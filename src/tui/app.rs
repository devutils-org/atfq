//! The interactive mode: state, keys, mouse and drawing. It never touches the
//! terminal itself, so tests drive it with plain events and a test backend.
//!
//! Every widget reports the regions it drew; a hit test on them is the only
//! mouse logic. Pages come from the same [`render`] functions as the CLI.

use std::collections::BTreeSet;

use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Constraint, Layout as Split, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line as TLine, Span as TSpan};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::theme;
use ratatui::Frame;

use crate::content::{ChapterKind, Library};
use crate::probes::ProbeCtx;
use crate::prompt::{self, SystemInfo};
use crate::render::{self, wrap, Action, DeepDive, Doc, Kind, Layout, Line, Span};
use crate::resolve::Target;
use crate::search::search;
use crate::view::{lookup, Lookup};
use crate::BIN_NAME;

/// Lines moved by one turn of the mouse wheel.
const WHEEL: usize = 3;

/// Search results shown in the TUI.
const SEARCH_LIMIT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Toc,
    Content,
}

/// What a click can land on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Pane(Pane),
    /// An entry of the table of contents, by index.
    TocItem(usize),
    /// A link or a deep dive question in the content pane, by index.
    Item(usize),
}

struct TocEntry {
    text: String,
    depth: usize,
    handle: Option<String>,
    header: bool,
}

/// Scroll position and selected link of a document pane.
#[derive(Debug, Default, Clone)]
struct DocView {
    scroll: usize,
    selected: Option<usize>,
    /// Scroll to the selection at the next draw.
    reveal: bool,
}

struct Page<'a> {
    handle: String,
    lookup: Lookup<'a>,
    expanded: BTreeSet<usize>,
    view: DocView,
}

/// A page in the back history.
struct Visit {
    handle: String,
    expanded: BTreeSet<usize>,
    view: DocView,
}

/// How far a vim motion goes.
#[derive(Debug, Clone, Copy)]
enum Motion {
    Lines(usize),
    HalfPages(usize),
    Pages(usize),
    Edge,
}

enum Mode {
    Normal,
    Search { query: String, view: DocView },
    Ask { text: String, view: DocView },
}

pub struct App<'a> {
    lib: &'a Library,
    ctx: ProbeCtx,
    sys: SystemInfo,
    toc: Vec<TocEntry>,
    toc_sel: usize,
    toc_scroll: usize,
    toc_reveal: bool,
    /// Height of the TOC pane at the last draw.
    toc_height: usize,
    page: Option<Page<'a>>,
    history: Vec<Visit>,
    focus: Pane,
    mode: Mode,
    mouse: bool,
    quit: bool,
    message: Option<String>,
    /// Vim prefixes typed so far: a count (`5j`) and the first `g` of `gg`.
    count: Option<usize>,
    pending_g: bool,
    regions: Vec<(Rect, Region)>,
    /// Links of the document drawn last: action and first line.
    items: Vec<(Action, usize)>,
    /// Height of the content pane at the last draw.
    content_height: usize,
}

impl<'a> App<'a> {
    pub fn new(lib: &'a Library, ctx: ProbeCtx, sys: SystemInfo) -> App<'a> {
        let toc = build_toc(lib);
        let toc_sel = toc.iter().position(|e| e.handle.is_some()).unwrap_or(0);
        App {
            lib,
            ctx,
            sys,
            toc,
            toc_sel,
            toc_scroll: 0,
            toc_reveal: true,
            toc_height: 0,
            page: None,
            history: Vec::new(),
            focus: Pane::Toc,
            mode: Mode::Normal,
            mouse: true,
            quit: false,
            message: None,
            count: None,
            pending_g: false,
            regions: Vec::new(),
            items: Vec::new(),
            content_height: 0,
        }
    }

    /// Opens what `query` resolves to, as `atfq <query>` would.
    pub fn open(&mut self, query: &str) -> bool {
        self.navigate(query)
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn mouse_captured(&self) -> bool {
        self.mouse
    }

    pub fn focus(&self) -> Pane {
        self.focus
    }

    /// Handle of the page shown (`memory`, `5.1`).
    pub fn current(&self) -> Option<&str> {
        self.page.as_ref().map(|p| p.handle.as_str())
    }

    pub fn content_scroll(&self) -> usize {
        match &self.mode {
            Mode::Normal => self.page.as_ref().map_or(0, |p| p.view.scroll),
            Mode::Search { view, .. } | Mode::Ask { view, .. } => view.scroll,
        }
    }

    pub fn regions(&self) -> &[(Rect, Region)] {
        &self.regions
    }

    /// Where the TOC entry for `handle` was drawn.
    pub fn toc_region(&self, handle: &str) -> Option<Rect> {
        let i = self
            .toc
            .iter()
            .position(|e| e.handle.as_deref() == Some(handle))?;
        self.region(Region::TocItem(i))
    }

    /// Where the first span of a link with this action was drawn.
    pub fn item_region(&self, action: &Action) -> Option<Rect> {
        let i = self.items.iter().position(|(a, _)| a == action)?;
        self.region(Region::Item(i))
    }

    fn region(&self, r: Region) -> Option<Rect> {
        self.regions
            .iter()
            .find(|(_, x)| *x == r)
            .map(|(rect, _)| *rect)
    }

    // ----- navigation -------------------------------------------------

    /// Loads a page. Probes run here, so `r` is a reload.
    fn load(&mut self, query: &str, expanded: BTreeSet<usize>, view: DocView) -> bool {
        let l = lookup(self.lib, query, &self.ctx);
        let Some(entry) = l.entry else {
            self.message = Some(format!("Nothing matches \"{query}\""));
            return false;
        };
        let handle = entry.handle();
        if let Some(i) = self
            .toc
            .iter()
            .position(|e| e.handle.as_ref() == Some(&handle))
        {
            self.toc_sel = i;
            self.toc_reveal = true;
        }
        self.page = Some(Page {
            handle,
            lookup: l,
            expanded,
            view,
        });
        true
    }

    fn navigate(&mut self, query: &str) -> bool {
        let visit = self.page.as_ref().map(|p| Visit {
            handle: p.handle.clone(),
            expanded: p.expanded.clone(),
            view: DocView {
                reveal: false,
                ..p.view.clone()
            },
        });
        if !self.load(query, BTreeSet::new(), DocView::default()) {
            return false;
        }
        self.history.extend(visit);
        self.mode = Mode::Normal;
        true
    }

    /// Closes an overlay, or goes back one page.
    fn back(&mut self) {
        if !matches!(self.mode, Mode::Normal) {
            self.mode = Mode::Normal;
            return;
        }
        if let Some(v) = self.history.pop() {
            self.load(&v.handle, v.expanded, v.view);
        }
    }

    fn refresh(&mut self) {
        let Some(p) = &self.page else { return };
        let (handle, expanded, view) = (p.handle.clone(), p.expanded.clone(), p.view.clone());
        self.load(&handle, expanded, view);
        self.message = Some("Probes refreshed".to_string());
    }

    fn activate(&mut self, action: Action) {
        match action {
            Action::Open(handle) => {
                self.navigate(&handle);
                self.focus = Pane::Content;
            }
            Action::Toggle(i) => {
                let line = self
                    .items
                    .iter()
                    .find_map(|(a, line)| (*a == Action::Toggle(i)).then_some(*line));
                if let Some(p) = self.page.as_mut() {
                    if !p.expanded.remove(&i) {
                        p.expanded.insert(i);
                        // Bring the question to the top so its answer shows.
                        if let Some(line) = line {
                            p.view.scroll = line;
                        }
                    }
                }
            }
        }
    }

    fn ask(&mut self) {
        let Some(p) = &self.page else {
            self.message = Some("Open a page first: the prompt is about it".to_string());
            return;
        };
        let prompt = prompt::build(&p.lookup, None, &self.sys, false);
        self.mode = Mode::Ask {
            text: prompt.text,
            view: DocView::default(),
        };
    }

    fn view_mut(&mut self) -> Option<&mut DocView> {
        match &mut self.mode {
            Mode::Normal => self.page.as_mut().map(|p| &mut p.view),
            Mode::Search { view, .. } | Mode::Ask { view, .. } => Some(view),
        }
    }

    fn scroll_content(&mut self, down: bool, n: usize) {
        if let Some(v) = self.view_mut() {
            v.scroll = if down {
                v.scroll + n
            } else {
                v.scroll.saturating_sub(n)
            };
        }
    }

    /// Selects the next (or previous) link of the content pane.
    fn step_link(&mut self, forward: bool) {
        let count = self.items.len();
        if count == 0 {
            return;
        }
        let first_visible = {
            let scroll = self.content_scroll();
            self.items
                .iter()
                .position(|(_, line)| *line >= scroll)
                .unwrap_or(0)
        };
        if let Some(v) = self.view_mut() {
            v.selected = Some(match v.selected {
                None => first_visible,
                Some(i) if forward => (i + 1) % count,
                Some(i) => (i + count - 1) % count,
            });
            v.reveal = true;
        }
        self.focus = Pane::Content;
    }

    fn activate_selected(&mut self) {
        let selected = match &self.mode {
            Mode::Search { view, .. } => view.selected.or(Some(0)),
            Mode::Normal => self.page.as_ref().and_then(|p| p.view.selected),
            Mode::Ask { .. } => None,
        };
        if let Some((action, _)) = selected.and_then(|i| self.items.get(i)).cloned() {
            self.activate(action);
        }
    }

    fn move_toc(&mut self, down: bool) {
        let mut i = self.toc_sel;
        loop {
            i = if down {
                i + 1
            } else {
                match i.checked_sub(1) {
                    Some(i) => i,
                    None => return,
                }
            };
            match self.toc.get(i) {
                None => return,
                Some(e) if e.handle.is_some() => {
                    self.toc_sel = i;
                    self.toc_reveal = true;
                    return;
                }
                Some(_) => {}
            }
        }
    }

    /// Selects the first (or last) page of the TOC.
    fn toc_edge(&mut self, last: bool) {
        let mut pages = self
            .toc
            .iter()
            .enumerate()
            .filter(|(_, e)| e.handle.is_some());
        let found = if last {
            pages.next_back()
        } else {
            pages.next()
        };
        if let Some((i, _)) = found {
            self.toc_sel = i;
            self.toc_reveal = true;
        }
    }

    /// Moves the TOC selection or scrolls the content, as `pane` says.
    fn motion(&mut self, pane: Pane, down: bool, m: Motion) {
        let height = match pane {
            Pane::Toc => self.toc_height,
            Pane::Content => self.content_height,
        };
        let n = match m {
            Motion::Lines(n) => n,
            Motion::HalfPages(n) => n * (height / 2).max(1),
            Motion::Pages(n) => n * height.saturating_sub(1).max(1),
            Motion::Edge => match pane {
                Pane::Toc => return self.toc_edge(down),
                Pane::Content => {
                    if let Some(v) = self.view_mut() {
                        v.scroll = if down { usize::MAX / 2 } else { 0 };
                    }
                    return;
                }
            },
        };
        match pane {
            Pane::Toc => {
                for _ in 0..n.min(self.toc.len()) {
                    self.move_toc(down);
                }
            }
            Pane::Content => self.scroll_content(down, n),
        }
    }

    /// Vim keys shared by the page and the prompt overlay: counts, `gg`,
    /// `G` and the Ctrl scrolls. Returns false when `k` is not one of them.
    fn vim_motion(&mut self, k: KeyEvent, pane: Pane) -> bool {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if let (KeyCode::Char(c @ '0'..='9'), false) = (k.code, ctrl) {
            // A lone 0 is not a count.
            if c != '0' || self.count.is_some() {
                let d = c as usize - '0' as usize;
                self.count = Some(self.count.unwrap_or(0).saturating_mul(10).saturating_add(d));
                self.pending_g = false;
                return true;
            }
        }
        let g = std::mem::take(&mut self.pending_g);
        // Esc first drops a half-typed count or g.
        if k.code == KeyCode::Esc && (g || self.count.is_some()) {
            self.count = None;
            return true;
        }
        let n = self.count.unwrap_or(1);
        let (down, m) = match (k.code, ctrl) {
            (KeyCode::Char('g'), false) if !g => {
                self.pending_g = true;
                return true;
            }
            (KeyCode::Char('g'), false) | (KeyCode::Home, _) => (false, Motion::Edge),
            (KeyCode::Char('G'), false) | (KeyCode::End, _) => (true, Motion::Edge),
            (KeyCode::Down | KeyCode::Char('j'), false) => (true, Motion::Lines(n)),
            (KeyCode::Up | KeyCode::Char('k'), false) => (false, Motion::Lines(n)),
            (KeyCode::Char('d'), true) => (true, Motion::HalfPages(n)),
            (KeyCode::Char('u'), true) => (false, Motion::HalfPages(n)),
            (KeyCode::PageDown | KeyCode::Char(' '), false) | (KeyCode::Char('f'), true) => {
                (true, Motion::Pages(n))
            }
            (KeyCode::PageUp, false) | (KeyCode::Char('b'), true) => (false, Motion::Pages(n)),
            // Ctrl-e and Ctrl-y scroll the page whatever the focus, as in vim.
            (KeyCode::Char('e'), true) => {
                self.count = None;
                self.scroll_content(true, n);
                return true;
            }
            (KeyCode::Char('y'), true) => {
                self.count = None;
                self.scroll_content(false, n);
                return true;
            }
            _ => return false,
        };
        self.count = None;
        self.motion(pane, down, m);
        true
    }

    fn open_toc(&mut self, i: usize) {
        if let Some(h) = self.toc.get(i).and_then(|e| e.handle.clone()) {
            self.navigate(&h);
        }
    }

    // ----- events -----------------------------------------------------

    pub fn handle(&mut self, event: Event) {
        match event {
            Event::Key(k) if k.kind == KeyEventKind::Press => {
                self.message = None;
                self.key(k);
            }
            Event::Mouse(m) if self.mouse => self.mouse_event(m),
            _ => {}
        }
    }

    fn key(&mut self, k: KeyEvent) {
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match &mut self.mode {
            Mode::Search { query, view } => {
                match k.code {
                    KeyCode::Char('n' | 'j') if ctrl => self.step_link(true),
                    KeyCode::Char('p' | 'k') if ctrl => self.step_link(false),
                    KeyCode::Char('u') if ctrl => {
                        query.clear();
                        *view = DocView::default();
                    }
                    KeyCode::Char('w') if ctrl => {
                        let kept = query.trim_end().len();
                        let start = query[..kept].rfind(' ').map_or(0, |i| i + 1);
                        query.truncate(start);
                        *view = DocView::default();
                    }
                    KeyCode::Char(_) if ctrl => {}
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter => self.activate_selected(),
                    KeyCode::Down | KeyCode::Tab => self.step_link(true),
                    KeyCode::Up | KeyCode::BackTab => self.step_link(false),
                    KeyCode::Backspace => {
                        query.pop();
                        *view = DocView::default();
                    }
                    KeyCode::Char(c) => {
                        query.push(c);
                        *view = DocView::default();
                    }
                    _ => {}
                }
                return;
            }
            Mode::Ask { .. } => {
                if self.vim_motion(k, Pane::Content) {
                    return;
                }
                self.count = None;
                match k.code {
                    KeyCode::Char('o') if ctrl => self.mode = Mode::Normal,
                    KeyCode::Esc | KeyCode::Char('?') => self.mode = Mode::Normal,
                    KeyCode::Char('q') => self.quit = true,
                    KeyCode::Char('M') => self.toggle_mouse(),
                    _ => {}
                }
                return;
            }
            Mode::Normal => {}
        }
        if self.vim_motion(k, self.focus) {
            return;
        }
        let count = self.count.take();
        match k.code {
            KeyCode::Char('o') if ctrl => self.back(),
            KeyCode::Char(_) if ctrl => {}
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('M') => self.toggle_mouse(),
            KeyCode::Char('/') => {
                self.mode = Mode::Search {
                    query: String::new(),
                    view: DocView::default(),
                };
                self.focus = Pane::Content;
            }
            KeyCode::Char('?') => self.ask(),
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Backspace => self.back(),
            KeyCode::Tab | KeyCode::BackTab => {
                for _ in 0..count.unwrap_or(1).min(self.items.len().max(1)) {
                    self.step_link(k.code == KeyCode::Tab);
                }
            }
            KeyCode::Left | KeyCode::Char('h') => self.focus = Pane::Toc,
            KeyCode::Right | KeyCode::Char('l') => self.focus = Pane::Content,
            KeyCode::Enter => match self.focus {
                Pane::Toc => self.open_toc(self.toc_sel),
                Pane::Content => self.activate_selected(),
            },
            _ => {}
        }
    }

    fn toggle_mouse(&mut self) {
        self.mouse = !self.mouse;
        self.message = Some(if self.mouse {
            "Mouse on: click to open, right click to go back".to_string()
        } else {
            "Mouse off: select text with the mouse, M to turn it back on".to_string()
        });
    }

    fn mouse_event(&mut self, m: MouseEvent) {
        let at = Position::new(m.column, m.row);
        // Last drawn wins: items are drawn over their pane.
        let hit = self
            .regions
            .iter()
            .rev()
            .find(|(r, _)| r.contains(at))
            .map(|(_, region)| *region);
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.message = None;
                if matches!(self.mode, Mode::Ask { .. }) {
                    return;
                }
                match hit {
                    Some(Region::TocItem(i)) => {
                        self.focus = Pane::Toc;
                        if self.toc_sel == i {
                            self.open_toc(i);
                        } else {
                            self.toc_sel = i;
                        }
                    }
                    Some(Region::Item(i)) => {
                        self.focus = Pane::Content;
                        let Some(view) = self.view_mut() else { return };
                        if view.selected == Some(i) {
                            if let Some((action, _)) = self.items.get(i).cloned() {
                                self.activate(action);
                            }
                        } else {
                            view.selected = Some(i);
                        }
                    }
                    Some(Region::Pane(p)) => self.focus = p,
                    None => {}
                }
            }
            MouseEventKind::Down(MouseButton::Right) => self.back(),
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                let down = m.kind == MouseEventKind::ScrollDown;
                match hit {
                    Some(Region::TocItem(_)) | Some(Region::Pane(Pane::Toc)) => {
                        self.toc_scroll = if down {
                            self.toc_scroll + WHEEL
                        } else {
                            self.toc_scroll.saturating_sub(WHEEL)
                        };
                    }
                    Some(_) => self.scroll_content(down, WHEEL),
                    None => {}
                }
            }
            // Moves, drags and button releases are ignored.
            _ => {}
        }
    }

    // ----- drawing ----------------------------------------------------

    pub fn draw(&mut self, f: &mut Frame<'_>) {
        self.regions.clear();
        let area = f.area();
        let [main, footer] =
            Split::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);
        let toc_width = (area.width / 3).clamp(20, 30);
        let [toc, content] =
            Split::horizontal([Constraint::Length(toc_width), Constraint::Min(1)]).areas(main);
        self.draw_toc(f, toc);
        self.draw_content(f, content);
        if matches!(self.mode, Mode::Ask { .. }) {
            self.draw_ask(f, main);
        }
        self.draw_footer(f, footer);
    }

    fn block(&self, title: String, focused: bool) -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(theme::BORDER)
            .border_style(theme::border(focused))
            .title(TSpan::styled(title, theme::title(focused)))
    }

    fn draw_toc(&mut self, f: &mut Frame<'_>, area: Rect) {
        let focused = self.focus == Pane::Toc && matches!(self.mode, Mode::Normal);
        let block = self.block(" Contents ".to_string(), focused);
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.regions.push((area, Region::Pane(Pane::Toc)));

        let height = inner.height as usize;
        self.toc_height = height;
        if self.toc_reveal {
            if self.toc_sel < self.toc_scroll {
                self.toc_scroll = self.toc_sel;
            } else if height > 0 && self.toc_sel >= self.toc_scroll + height {
                self.toc_scroll = self.toc_sel + 1 - height;
            }
            self.toc_reveal = false;
        }
        self.toc_scroll = self.toc_scroll.min(self.toc.len().saturating_sub(height));

        let current = self.current().map(str::to_string);
        let width = inner.width as usize;
        let mut lines = Vec::new();
        for (row, i) in (self.toc_scroll..self.toc.len()).take(height).enumerate() {
            let e = &self.toc[i];
            let text = truncate(&format!("{}{}", "  ".repeat(e.depth), e.text), width);
            let mut style = Style::default();
            if e.header {
                style = style.add_modifier(Modifier::BOLD);
            } else if e.handle.is_none() {
                style = style.fg(theme::MUTED);
            }
            if e.handle.is_some() && e.handle == current {
                style = style.fg(theme::ACCENT).add_modifier(Modifier::BOLD);
            }
            if i == self.toc_sel && focused {
                style = theme::selection();
            }
            if e.handle.is_some() {
                let rect = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
                self.regions.push((rect, Region::TocItem(i)));
            }
            lines.push(TLine::styled(format!("{text:<width$}"), style));
        }
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn draw_content(&mut self, f: &mut Frame<'_>, area: Rect) {
        let focused = self.focus == Pane::Content || !matches!(self.mode, Mode::Normal);
        let title = match (&self.mode, &self.page) {
            (Mode::Search { .. }, _) => " Search ".to_string(),
            (_, Some(p)) => match p.lookup.entry {
                Some(Target::Card(c)) => format!(" {} {} ", c.number, c.title),
                Some(Target::Hub(h)) => format!(" {} ", h.title),
                None => " atfq ".to_string(),
            },
            (_, None) => " atfq ".to_string(),
        };
        let block = self.block(title, focused);
        let mut inner = block.inner(area);
        f.render_widget(block, area);
        self.regions.push((area, Region::Pane(Pane::Content)));
        let width = inner.width as usize;

        let lib = self.lib;
        let (doc, view) = match &mut self.mode {
            Mode::Search { query, view } => {
                let input = TLine::from(vec![
                    TSpan::styled("/ ", Style::default().fg(theme::SEARCH)),
                    TSpan::raw(query.clone()),
                    TSpan::styled("█", theme::muted()),
                ]);
                f.render_widget(Paragraph::new(input), Rect { height: 1, ..inner });
                inner.y += 2.min(inner.height);
                inner.height = inner.height.saturating_sub(2);
                let doc = if query.trim().is_empty() {
                    Doc {
                        lines: vec![Line::styled(
                            "Type words to search every hub and card.",
                            Kind::Dim,
                        )],
                    }
                } else {
                    render::search_doc(query, &search(lib, query, SEARCH_LIMIT), width)
                };
                (doc, view)
            }
            Mode::Normal | Mode::Ask { .. } => match self.page.as_mut() {
                Some(p) => {
                    let layout = Layout {
                        width,
                        deep_dive: DeepDive::Toggle(p.expanded.clone()),
                    };
                    let doc = render::lookup_doc(lib, &p.lookup, &layout).unwrap_or_default();
                    (doc, &mut p.view)
                }
                None => {
                    self.content_height = inner.height as usize;
                    let mut view = DocView::default();
                    self.items =
                        draw_doc(f, inner, &welcome(lib, width), &mut view, &mut self.regions);
                    return;
                }
            },
        };
        self.content_height = inner.height as usize;
        self.items = draw_doc(f, inner, &doc, view, &mut self.regions);
    }

    fn draw_ask(&mut self, f: &mut Frame<'_>, area: Rect) {
        let Mode::Ask { text, view } = &mut self.mode else {
            return;
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(theme::BORDER)
            .border_style(Style::default().fg(theme::PROMPT))
            .title(TSpan::styled(
                " Prompt for an AI assistant (same as atfq ask) ",
                theme::title(true).fg(theme::PROMPT),
            ));
        let inner = block.inner(area);
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let width = inner.width as usize;
        let mut lines = Vec::new();
        for l in text.lines() {
            let wrapped = wrap(&[Span::plain(l)], &Line::default(), &Line::default(), width);
            if wrapped.is_empty() {
                lines.push(Line::default());
            }
            lines.extend(wrapped);
        }
        // The prompt has no links: its regions only cover the pane.
        self.regions.push((area, Region::Pane(Pane::Content)));
        self.content_height = inner.height as usize;
        draw_doc(f, inner, &Doc { lines }, view, &mut self.regions);
    }

    fn draw_footer(&mut self, f: &mut Frame<'_>, area: Rect) {
        let (mode, color) = match self.mode {
            Mode::Normal => ("NORMAL", theme::NORMAL),
            Mode::Search { .. } => ("SEARCH", theme::SEARCH),
            Mode::Ask { .. } => ("PROMPT", theme::PROMPT),
        };
        let mouse = if self.mouse { "mouse:on" } else { "mouse:off" };
        let keys: &[(&str, &str)] = match self.mode {
            Mode::Normal => &[
                ("q", "quit"),
                ("jk", "move"),
                ("tab", "link"),
                ("⏎", "open"),
                ("⌫", "back"),
                ("/", "search"),
                ("?", "AI"),
                ("r", "refresh"),
                ("←→", "pane"),
            ],
            Mode::Search { .. } => &[
                ("esc", "close"),
                ("↑↓", "result"),
                ("⏎", "open"),
                ("⌫", "erase"),
            ],
            Mode::Ask { .. } => &[("esc", "close"), ("↑↓", "scroll"), ("q", "quit")],
        };
        let width = area.width as usize;
        let mut spans = vec![
            TSpan::styled(format!(" {mode} "), theme::badge(color)),
            TSpan::raw(" "),
        ];
        // The mouse state always shows, on the right; the other keys while
        // they fit.
        let tail = vec![
            TSpan::styled("M", theme::key()),
            TSpan::styled(format!(" {mouse} "), theme::muted()),
        ];
        let mut used = mode.len() + 3 + 3 + mouse.len();
        // A half-typed vim command (5, g), like vim's showcmd.
        let mut pending = self.count.map(|n| n.to_string()).unwrap_or_default();
        if self.pending_g {
            pending.push('g');
        }
        if !pending.is_empty() {
            used += pending.chars().count() + 2;
            spans.push(TSpan::styled(pending, theme::key().fg(color)));
            spans.push(TSpan::raw("  "));
        }
        if let Some(msg) = &self.message {
            let msg = truncate(msg, width.saturating_sub(used + 1));
            used += msg.chars().count();
            spans.push(TSpan::styled(msg, Style::default().fg(theme::WARN)));
        } else {
            for (k, l) in keys {
                let w = k.chars().count() + l.chars().count() + 3;
                if used + w > width {
                    break;
                }
                used += w;
                spans.push(TSpan::styled(*k, theme::key()));
                spans.push(TSpan::styled(format!(" {l}  "), theme::muted()));
            }
        }
        spans.push(TSpan::raw(" ".repeat(width.saturating_sub(used))));
        spans.extend(tail);
        f.render_widget(Paragraph::new(TLine::from(spans)), area);
    }
}

/// Draws `doc` in `area` at the view's scroll, records a region per link
/// span, and returns the links: action and first line.
fn draw_doc(
    f: &mut Frame<'_>,
    area: Rect,
    doc: &Doc,
    view: &mut DocView,
    regions: &mut Vec<(Rect, Region)>,
) -> Vec<(Action, usize)> {
    let (items, map) = links(doc);
    let height = area.height as usize;
    if view.selected.is_some_and(|s| s >= items.len()) {
        view.selected = None;
    }
    if view.reveal {
        if let Some(line) = view.selected.map(|s| items[s].1) {
            if line < view.scroll {
                view.scroll = line;
            } else if height > 0 && line >= view.scroll + height {
                view.scroll = line + 1 - height;
            }
        }
        view.reveal = false;
    }
    view.scroll = view.scroll.min(doc.lines.len().saturating_sub(height));

    let mut lines = Vec::new();
    for (row, li) in (view.scroll..doc.lines.len()).take(height).enumerate() {
        let mut x: u16 = 0;
        let mut spans = Vec::new();
        for (si, span) in doc.lines[li].spans.iter().enumerate() {
            let item = map[li][si];
            let mut style = theme::of(span.kind);
            if item.is_some() {
                style = style.add_modifier(Modifier::UNDERLINED);
                if item == view.selected {
                    style = style.add_modifier(Modifier::REVERSED);
                }
            }
            let w = render::width(&span.text) as u16;
            if let Some(i) = item {
                if x < area.width {
                    let rect = Rect::new(area.x + x, area.y + row as u16, w.min(area.width - x), 1);
                    regions.push((rect, Region::Item(i)));
                }
            }
            x = x.saturating_add(w);
            spans.push(TSpan::styled(span.text.clone(), style));
        }
        lines.push(TLine::from(spans));
    }
    f.render_widget(Paragraph::new(lines), area);
    items
}

/// Groups actionable spans into links. Neighbouring spans with the same
/// action form one link, even across a line break (a wrapped title).
#[allow(clippy::type_complexity)]
fn links(doc: &Doc) -> (Vec<(Action, usize)>, Vec<Vec<Option<usize>>>) {
    let mut items: Vec<(Action, usize)> = Vec::new();
    let mut map = Vec::new();
    let mut current: Option<usize> = None;
    for (li, line) in doc.lines.iter().enumerate() {
        let mut row = Vec::new();
        for span in &line.spans {
            match &span.action {
                Some(a) => {
                    if !current.is_some_and(|c| &items[c].0 == a) {
                        items.push((a.clone(), li));
                        current = Some(items.len() - 1);
                    }
                    row.push(current);
                }
                None => {
                    if !span.text.trim().is_empty() {
                        current = None;
                    }
                    row.push(None);
                }
            }
        }
        map.push(row);
    }
    (items, map)
}

fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        return s.to_string();
    }
    let mut t: String = s.chars().take(width.saturating_sub(1)).collect();
    t.push('…');
    t
}

fn build_toc(lib: &Library) -> Vec<TocEntry> {
    let entry = |text: String, depth, handle: Option<String>| TocEntry {
        text,
        depth,
        handle,
        header: false,
    };
    let header = |text: &str| TocEntry {
        text: text.to_string(),
        depth: 0,
        handle: None,
        header: true,
    };
    let mut toc = vec![header("Hubs")];
    for h in lib.hubs() {
        toc.push(entry(h.title.clone(), 1, Some(h.id.clone())));
    }
    let mut part: Option<&str> = None;
    let mut parts = 0;
    let mut first = true;
    for ch in lib.chapters() {
        if first || ch.part.as_deref() != part {
            first = false;
            part = ch.part.as_deref();
            toc.push(match part {
                Some(title) => {
                    parts += 1;
                    header(&format!("{}. {title}", render::roman(parts)))
                }
                None => header("Chapters"),
            });
        }
        let handle = match &ch.kind {
            ChapterKind::Hub(id) => Some(id.clone()),
            ChapterKind::Plain => None,
        };
        toc.push(entry(format!("{}. {}", ch.number, ch.title), 0, handle));
        for id in &ch.cards {
            if let Some(c) = lib.card(id) {
                let n = c.number.to_string();
                toc.push(entry(format!("{n} {}", c.title), 1, Some(n)));
            }
        }
    }
    toc
}

/// The page shown before anything is opened.
fn welcome(lib: &Library, width: usize) -> Doc {
    let mut lines = vec![Line::styled("Welcome to atfq", Kind::Bold), Line::default()];
    lines.extend(wrap(
        &[Span::plain(
            "Pick a hub or a chapter on the left, or press / to search. \
             Hubs are the best place to start:",
        )],
        &Line::default(),
        &Line::default(),
        width,
    ));
    lines.push(Line::default());
    let left = lib
        .hubs()
        .map(|h| render::width(&h.id) + BIN_NAME.len() + 1)
        .max()
        .unwrap_or(0);
    for h in lib.hubs() {
        let open = Action::Open(h.id.clone());
        let cmd = format!("{BIN_NAME} {}", h.id);
        let pad = left - render::width(&cmd) + 2;
        lines.push(Line::new(vec![
            Span::plain("  "),
            Span::new(cmd, Kind::Code).with(open.clone()),
            Span::plain(" ".repeat(pad)),
            Span::plain(&h.title).with(open),
        ]));
    }
    Doc { lines }
}
