//! Phase 2 acceptance: the TUI, driven by synthetic key and mouse events on
//! an 80×24 test backend. No real terminal involved.

mod common;

use atfq::content::Library;
use atfq::prompt::SystemInfo;
use atfq::render::Action;
use atfq::tui::{App, Pane};
use common::{laptop, lib};
use insta::assert_snapshot;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;
use ratatui::Terminal;

fn sys() -> SystemInfo {
    SystemInfo {
        distro: Some("Debian GNU/Linux 13 (trixie)".into()),
        kernel: Some("6.12.48-amd64".into()),
        arch: "x86_64".into(),
        hostname: Some("alice-laptop".into()),
        user: Some("alice".into()),
        home: Some("/home/alice".into()),
    }
}

struct Tui<'a> {
    app: App<'a>,
    term: Terminal<TestBackend>,
}

impl<'a> Tui<'a> {
    fn new(lib: &'a Library) -> Tui<'a> {
        let mut t = Tui {
            app: App::new(lib, laptop(), sys()),
            term: Terminal::new(TestBackend::new(80, 24)).unwrap(),
        };
        t.draw();
        t
    }

    fn draw(&mut self) {
        let app = &mut self.app;
        self.term.draw(|f| app.draw(f)).unwrap();
    }

    fn screen(&self) -> String {
        self.term.backend().to_string()
    }

    fn send(&mut self, e: Event) {
        self.app.handle(e);
        self.draw();
    }

    fn key(&mut self, code: KeyCode) {
        self.send(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }

    fn typing(&mut self, text: &str) {
        for c in text.chars() {
            self.key(KeyCode::Char(c));
        }
    }

    fn mouse(&mut self, kind: MouseEventKind, x: u16, y: u16) {
        self.send(Event::Mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }));
    }

    fn click(&mut self, r: Rect) {
        self.mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y);
    }

    /// Click to select, click again to go in.
    fn double(&mut self, r: Rect) {
        self.click(r);
        self.click(r);
    }

    fn right_click(&mut self) {
        self.mouse(MouseEventKind::Down(MouseButton::Right), 40, 10);
    }

    /// Wheels down over the TOC until the entry for `handle` is on screen.
    fn toc_entry(&mut self, handle: &str) -> Rect {
        for _ in 0..30 {
            if let Some(r) = self.app.toc_region(handle) {
                return r;
            }
            self.mouse(MouseEventKind::ScrollDown, 2, 10);
        }
        panic!("{handle} never showed up in the TOC:\n{}", self.screen());
    }

    /// Wheels down over the content pane until `action` is on screen.
    fn scroll_to(&mut self, action: &Action) -> Rect {
        for _ in 0..30 {
            if let Some(r) = self.app.item_region(action) {
                return r;
            }
            self.mouse(MouseEventKind::ScrollDown, 50, 10);
        }
        panic!("{action:?} never showed up:\n{}", self.screen());
    }
}

#[test]
fn start_screen() {
    let lib = lib();
    let t = Tui::new(&lib);
    assert_snapshot!("tui_start", t.screen());
}

#[test]
fn scripted_clicks() {
    let lib = lib();
    let mut t = Tui::new(&lib);

    // A TOC entry: the first click selects, the second opens.
    let entry = t.toc_entry("13.1");
    t.click(entry);
    assert_eq!(t.app.current(), None);
    t.click(entry);
    assert_eq!(t.app.current(), Some("13.1"));
    assert_snapshot!("tui_card", t.screen());

    // A deep dive question opens in place.
    let q = t.scroll_to(&Action::Toggle(0));
    t.double(q);
    assert!(
        t.screen().contains("▾ Why is unused RAM wasted RAM?"),
        "{}",
        t.screen()
    );
    assert!(t.screen().contains("RAM is much faster than any disk"));
    assert_snapshot!("tui_question_open", t.screen());

    // A link to the hub.
    let link = t.scroll_to(&Action::Open("memory".into()));
    t.double(link);
    assert_eq!(t.app.current(), Some("memory"));
    assert_snapshot!("tui_hub", t.screen());

    // Right click goes back, with the question still open.
    t.right_click();
    assert_eq!(t.app.current(), Some("13.1"));
    assert!(t.screen().contains("▾ Why is unused RAM wasted RAM?"));

    // Nothing before 5.1: right click stays there.
    t.right_click();
    assert_eq!(t.app.current(), Some("13.1"));
}

#[test]
fn wheel_scrolls_the_pane_under_the_pointer() {
    let lib = lib();
    let mut t = Tui::new(&lib);
    t.app.open("12.1");
    t.draw();
    assert_eq!(t.app.content_scroll(), 0);
    t.mouse(MouseEventKind::ScrollDown, 50, 10);
    assert_eq!(t.app.content_scroll(), 3);
    t.mouse(MouseEventKind::ScrollUp, 50, 10);
    assert_eq!(t.app.content_scroll(), 0);
    // Moves, drags and releases do nothing.
    let before = t.screen();
    t.mouse(MouseEventKind::Moved, 5, 5);
    t.mouse(MouseEventKind::Drag(MouseButton::Left), 5, 5);
    t.mouse(MouseEventKind::Up(MouseButton::Left), 5, 5);
    assert_eq!(t.screen(), before);
}

#[test]
fn clicking_a_pane_focuses_it() {
    let lib = lib();
    let mut t = Tui::new(&lib);
    assert_eq!(t.app.focus(), Pane::Toc);
    t.mouse(MouseEventKind::Down(MouseButton::Left), 70, 20);
    assert_eq!(t.app.focus(), Pane::Content);
    t.mouse(MouseEventKind::Down(MouseButton::Left), 2, 22);
    assert_eq!(t.app.focus(), Pane::Toc);
}

#[test]
fn keyboard_only() {
    let lib = lib();
    let mut t = Tui::new(&lib);
    // TOC hubs: Disk, Permissions, Processes, Memory…
    for _ in 0..3 {
        t.key(KeyCode::Down);
    }
    t.key(KeyCode::Enter);
    assert_eq!(t.app.current(), Some("memory"));

    // Tab walks the links of the page; Enter follows.
    t.key(KeyCode::Tab);
    assert_eq!(t.app.focus(), Pane::Content);
    t.key(KeyCode::Enter);
    assert_eq!(t.app.current(), Some("13.1"));
    t.key(KeyCode::Backspace);
    assert_eq!(t.app.current(), Some("memory"));

    // Search.
    t.key(KeyCode::Char('/'));
    t.typing("zomb");
    assert!(t.screen().contains("Process states and zombies"));
    assert_snapshot!("tui_search", t.screen());
    t.key(KeyCode::Enter);
    assert_eq!(t.app.current(), Some("12.1"));

    // Deep dive by keyboard: tab to the first question, enter to open.
    for _ in 0..10 {
        t.key(KeyCode::Tab);
        if t.screen().contains("▸ Why do zombies exist at all?") {
            break;
        }
    }
    t.key(KeyCode::Enter);
    assert!(
        t.screen().contains("▾ Why do zombies exist at all?"),
        "{}",
        t.screen()
    );

    t.key(KeyCode::Char('r'));
    assert!(t.screen().contains("Probes refreshed"));

    t.key(KeyCode::Char('q'));
    assert!(t.app.should_quit());
}

#[test]
fn escape_quits_or_closes() {
    let lib = lib();
    let mut t = Tui::new(&lib);
    t.key(KeyCode::Char('/'));
    t.key(KeyCode::Esc);
    assert!(!t.app.should_quit(), "Esc first closes the search");
    t.key(KeyCode::Esc);
    assert!(t.app.should_quit());
}

#[test]
fn mouse_toggle() {
    let lib = lib();
    let mut t = Tui::new(&lib);
    assert!(t.screen().contains("mouse:on"));
    let entry = t.toc_entry("13.1");
    t.key(KeyCode::Char('M'));
    assert!(!t.app.mouse_captured());
    assert!(t.screen().contains("mouse:off"));
    // With the mouse off, clicks are ignored.
    t.double(entry);
    assert_eq!(t.app.current(), None);
    t.key(KeyCode::Char('M'));
    assert!(t.app.mouse_captured());
}

#[test]
fn ask_overlay() {
    let lib = lib();
    let mut t = Tui::new(&lib);
    t.key(KeyCode::Char('?'));
    assert!(t.screen().contains("Open a page first"));
    t.app.open("memory");
    t.key(KeyCode::Char('?'));
    assert!(t.screen().contains("Prompt for an AI assistant"));
    assert!(t.screen().contains("## My question"));
    assert_snapshot!("tui_ask", t.screen());
    t.key(KeyCode::Esc);
    assert!(!t.screen().contains("Prompt for an AI assistant"));
    assert!(!t.app.should_quit());
}

#[test]
fn small_terminal_does_not_panic() {
    let lib = lib();
    let mut app = App::new(&lib, laptop(), sys());
    app.open("disk");
    for (w, h) in [(80, 24), (40, 10), (20, 5), (1, 1), (0, 0)] {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| app.draw(f)).unwrap();
    }
}
