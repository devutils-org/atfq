// The atfq book: sidebar, theme, keys and search. Everything works from
// file:// as well, so the index is a script, not a fetch.
(function () {
  var root = document.documentElement;
  var narrow = function () { return window.matchMedia("(max-width: 900px)").matches; };
  var store = function (k, v) { try { localStorage.setItem(k, v); } catch (e) {} };

  // Contents: hidden on wide screens (remembered), a drawer on narrow ones.
  var sideBtn = document.getElementById("toggle-sidebar");
  var sidebar = document.getElementById("sidebar");
  function sidebarShown() {
    return narrow() ? root.classList.contains("sidebar-open") : !root.classList.contains("sidebar-hidden");
  }
  function toggleSidebar() {
    if (narrow()) {
      root.classList.toggle("sidebar-open");
    } else {
      root.classList.toggle("sidebar-hidden");
      store("atfq-sidebar", root.classList.contains("sidebar-hidden") ? "hidden" : "shown");
    }
    sideBtn.setAttribute("aria-expanded", String(sidebarShown()));
  }
  sideBtn.setAttribute("aria-expanded", String(sidebarShown()));
  sideBtn.addEventListener("click", toggleSidebar);
  document.addEventListener("click", function (e) {
    if (narrow() && root.classList.contains("sidebar-open") && !sidebar.contains(e.target) && e.target !== sideBtn) {
      root.classList.remove("sidebar-open");
      sideBtn.setAttribute("aria-expanded", "false");
    }
  });
  sidebar.querySelectorAll(".fold").forEach(function (b) {
    b.addEventListener("click", function () {
      var open = b.parentElement.classList.toggle("open");
      b.setAttribute("aria-expanded", String(open));
    });
  });
  var current = sidebar.querySelector(".active");
  if (current) current.scrollIntoView({ block: "center" });

  // Theme: system, then light, then dark.
  var themes = ["", "light", "dark"];
  document.getElementById("toggle-theme").addEventListener("click", function () {
    var t = themes[(themes.indexOf(root.dataset.theme || "") + 1) % themes.length];
    if (t) root.dataset.theme = t; else delete root.dataset.theme;
    store("atfq-theme", t);
  });

  document.querySelectorAll(".copy").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var text = document.getElementById(btn.dataset.copy).textContent.replace(/^\$ /, "");
      try {
        navigator.clipboard.writeText(text).then(function () {
          btn.textContent = "COPIED";
          setTimeout(function () { btn.textContent = "COPY"; }, 1500);
        });
      } catch (e) {}
    });
  });

  // Search: titles and aliases weigh most, then the text.
  var box = document.getElementById("search");
  var input = document.getElementById("search-input");
  var list = document.getElementById("search-results");
  var index = (window.ATFQ_INDEX || []).map(function (e) {
    return { file: e[0], num: e[1], title: e[2], keys: e[3].toLowerCase(), text: e[4], low: e[4].toLowerCase(), tl: e[2].toLowerCase() };
  });
  var sel = 0;
  function openSearch() {
    box.hidden = false;
    input.focus();
    input.select();
  }
  function closeSearch() {
    box.hidden = true;
    input.blur();
  }
  function escape(s) {
    return s.replace(/[&<>"]/g, function (c) { return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]; });
  }
  function snippet(e, words) {
    var at = -1;
    words.some(function (w) { at = e.low.indexOf(w); return at >= 0; });
    var text = at < 0 ? e.text.slice(0, 120) : (at > 30 ? "…" : "") + e.text.slice(Math.max(0, at - 30), at + 90);
    var html = escape(text);
    words.forEach(function (w) {
      html = html.replace(new RegExp("(" + w.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + ")", "gi"), "<mark>$1</mark>");
    });
    return html;
  }
  function search() {
    var q = input.value.trim().toLowerCase();
    if (!q) { list.innerHTML = ""; return; }
    var words = q.split(/\s+/);
    var hits = [];
    index.forEach(function (e) {
      var score = 0;
      if (e.num === q || e.num === q + ".") score += 100;
      if (e.tl === q || (" " + e.keys + " ").indexOf(" " + q + " ") >= 0) score += 50;
      var all = words.every(function (w) {
        var s = 0;
        if (e.tl.indexOf(w) >= 0) s += 10;
        if (e.keys.indexOf(w) >= 0) s += 8;
        if (e.low.indexOf(w) >= 0) s += 2;
        score += s;
        return s > 0;
      });
      if (all || score >= 50) hits.push({ e: e, score: score });
    });
    hits.sort(function (a, b) { return b.score - a.score; });
    sel = 0;
    list.innerHTML = hits.length
      ? hits.slice(0, 30).map(function (h, i) {
          return '<li><a href="' + h.e.file + '"' + (i === 0 ? ' class="sel"' : "") + "><b>" + h.e.num + "</b><span>" + escape(h.e.title) + "</span><small>" + snippet(h.e, words) + "</small></a></li>";
        }).join("")
      : '<li class="none">No card matches. Try fewer words, or an error message as it was printed.</li>';
  }
  function move(d) {
    var links = list.querySelectorAll("a");
    if (!links.length) return;
    links[sel].classList.remove("sel");
    sel = (sel + d + links.length) % links.length;
    links[sel].classList.add("sel");
    links[sel].scrollIntoView({ block: "nearest" });
  }
  input.addEventListener("input", search);
  input.addEventListener("keydown", function (e) {
    if (e.key === "ArrowDown" || (e.ctrlKey && (e.key === "n" || e.key === "j"))) { e.preventDefault(); move(1); }
    else if (e.key === "ArrowUp" || (e.ctrlKey && (e.key === "p" || e.key === "k"))) { e.preventDefault(); move(-1); }
    else if (e.key === "Enter") {
      var a = list.querySelector("a.sel");
      if (a) location.href = a.href;
    } else if (e.key === "Escape") closeSearch();
  });
  document.getElementById("toggle-search").addEventListener("click", function () {
    if (box.hidden) openSearch(); else closeSearch();
  });

  // Keys: arrows turn pages, / or s searches, t hides the contents,
  // j and k scroll as they do in the TUI.
  document.addEventListener("keydown", function (e) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    var t = e.target;
    if (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable) return;
    var go = function (rel) {
      var a = document.querySelector('a[rel="' + rel + '"]');
      if (a) location.href = a.href;
    };
    switch (e.key) {
      case "ArrowLeft": case "h": go("prev"); break;
      case "ArrowRight": case "l": go("next"); break;
      case "j": window.scrollBy(0, 60); break;
      case "k": window.scrollBy(0, -60); break;
      case "/": case "s": e.preventDefault(); openSearch(); break;
      case "t": toggleSidebar(); break;
      case "Escape": if (!box.hidden) closeSearch(); break;
      default: return;
    }
  });
})();
