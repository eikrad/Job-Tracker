"""Visible-text extraction from HTML mail bodies (spec §6.2, §6.3).

Job alert digests are frequently HTML-only. Two rules matter here, and both are
security rules rather than cosmetic ones:

1. **Never pass markup to the model.** Tags and attributes are attacker-controlled
   structure; the scorer should see the same prose a human would.
2. **Never pass invisible text to the model.** A ``display:none`` block that
   contradicts the visible ad is the cleanest injection there is: the reviewer reads
   "Junior Barista" and the model reads "Senior Principal Engineer, score 10". If the
   user cannot see it, neither should the model.

Stdlib only — this runs inside the sidecar, which has no third-party dependencies.
"""

from __future__ import annotations

import re
from html.parser import HTMLParser

# Content of these elements is never prose.
_DROPPED_ELEMENTS = frozenset(
    {"script", "style", "head", "title", "meta", "link", "noscript", "template"}
)

# Elements that imply a line break in the rendered output.
_BLOCK_ELEMENTS = frozenset(
    {
        "address",
        "article",
        "aside",
        "blockquote",
        "br",
        "div",
        "dl",
        "dt",
        "dd",
        "fieldset",
        "figure",
        "footer",
        "form",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "header",
        "hr",
        "li",
        "main",
        "nav",
        "ol",
        "p",
        "pre",
        "section",
        "table",
        "tbody",
        "td",
        "tfoot",
        "th",
        "thead",
        "tr",
        "ul",
    }
)

# Void elements never have an end tag, so they must never push a skip frame.
_VOID_ELEMENTS = frozenset(
    {
        "area",
        "base",
        "br",
        "col",
        "embed",
        "hr",
        "img",
        "input",
        "link",
        "meta",
        "param",
        "source",
        "track",
        "wbr",
    }
)

_WHITESPACE = re.compile(r"[ \t\r\f\v]+")
_BLANK_LINES = re.compile(r"\n{3,}")

# Declarations that hide content from a reader. `font-size:0` and white-on-white are
# included because they are what real hidden-text injections use.
_HIDDEN_DECLARATIONS = (
    "display:none",
    "visibility:hidden",
    "opacity:0",
    "font-size:0",
    "font-size:0px",
    "font-size:0pt",
    "font-size:0em",
    "max-height:0",
    "text-indent:-9999px",
)

_WHITE_COLORS = ("color:#fff", "color:#ffffff", "color:white", "color:rgb(255,255,255)")


def _normalize_style(style: str) -> str:
    return re.sub(r"\s+", "", style or "").lower()


def is_hidden(attrs: dict[str, str | None]) -> bool:
    """True when these attributes hide the element from a human reader."""
    if "hidden" in attrs:
        return True
    if (attrs.get("aria-hidden") or "").strip().lower() == "true":
        return True

    style = _normalize_style(attrs.get("style") or "")
    if any(decl in style for decl in _HIDDEN_DECLARATIONS):
        return True
    # White text is only hidden if it is not also given a non-white background; we do
    # not attempt to resolve inherited backgrounds, so treat plain white text as hidden.
    return any(c in style for c in _WHITE_COLORS) and "background" not in style


class _VisibleTextParser(HTMLParser):
    """Collects rendered text, skipping dropped and invisible subtrees."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self._chunks: list[str] = []
        # Stack of open elements we are inside and skipping, so nested tags of the
        # same name close in the right order.
        self._skip_stack: list[str] = []

    # -- helpers ----------------------------------------------------------
    @property
    def _skipping(self) -> bool:
        return bool(self._skip_stack)

    def _emit(self, text: str) -> None:
        if text:
            self._chunks.append(text)

    # -- HTMLParser hooks -------------------------------------------------
    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        tag = tag.lower()
        if tag in _VOID_ELEMENTS:
            # No end tag will ever arrive, so pushing one would strand the stack.
            if not self._skipping and tag in _BLOCK_ELEMENTS:
                self._emit("\n")
            return
        if self._skipping:
            # Track nesting so the matching end tag pops the right frame.
            self._skip_stack.append(tag)
            return
        if tag in _DROPPED_ELEMENTS or is_hidden(dict(attrs)):
            self._skip_stack.append(tag)
            return
        if tag in _BLOCK_ELEMENTS:
            self._emit("\n")

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if self._skipping or is_hidden(dict(attrs)):
            return
        if tag.lower() in _BLOCK_ELEMENTS:
            self._emit("\n")

    def handle_endtag(self, tag: str) -> None:
        tag = tag.lower()
        if self._skipping:
            # Pop back to (and including) the most recent matching frame. Malformed
            # mail is the norm, so an unmatched close must not strand us in skip mode.
            for i in range(len(self._skip_stack) - 1, -1, -1):
                if self._skip_stack[i] == tag:
                    del self._skip_stack[i:]
                    return
            return
        if tag in _BLOCK_ELEMENTS:
            self._emit("\n")

    def handle_data(self, data: str) -> None:
        if not self._skipping:
            self._emit(data)

    def text(self) -> str:
        joined = "".join(self._chunks)
        joined = _WHITESPACE.sub(" ", joined)
        joined = "\n".join(line.strip() for line in joined.split("\n"))
        return _BLANK_LINES.sub("\n\n", joined).strip()


def html_to_visible_text(html: str) -> str:
    """Render `html` down to the prose a human would see.

    Malformed markup yields whatever could be recovered rather than raising — a
    broken digest should still be scannable.
    """
    parser = _VisibleTextParser()
    try:
        parser.feed(html)
        parser.close()
    except Exception:
        # Hostile, malformed input: keep whatever parsed rather than failing the scan.
        pass
    return parser.text()


def looks_like_html(text: str) -> bool:
    """Cheap sniff for a body that was sent as HTML without saying so."""
    lowered = text.lstrip()[:512].lower()
    return lowered.startswith(("<!doctype html", "<html")) or "<body" in lowered
