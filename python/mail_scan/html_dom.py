"""A small, forgiving element tree for alert-mail HTML.

Board extractors need structure — "the ``h4`` link inside this listing block", "the
first paragraph after the title" — that the flat visible-text rendering throws away.
Stdlib only (the sidecar has no third-party dependencies), and tolerant of the
malformed markup job-alert mail is made of: an unmatched end tag is ignored, an
unclosed element is closed by its parent's end tag.

Text read from the tree goes through the same visibility rules as
``html_text.html_to_visible_text``: hidden and non-prose subtrees are skipped, so a
snippet can never carry text a human reader of the mail could not see (spec §6.2).
"""

from __future__ import annotations

import re
from collections.abc import Callable, Iterator
from dataclasses import dataclass, field
from html.parser import HTMLParser

from mail_scan.html_text import (
    BLOCK_ELEMENTS,
    DROPPED_ELEMENTS,
    VOID_ELEMENTS,
    is_hidden,
    normalize_rendered_text,
)

# Starting any of these closes an open <p> (HTML's implied end tag).
_CLOSES_P = frozenset(
    {
        "address",
        "article",
        "aside",
        "blockquote",
        "div",
        "dl",
        "fieldset",
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
        "nav",
        "ol",
        "p",
        "pre",
        "section",
        "table",
        "ul",
    }
)


@dataclass(eq=False)
class Element:
    tag: str
    attrs: dict[str, str] = field(default_factory=dict)
    children: list[Element | str] = field(default_factory=list)
    parent: Element | None = field(default=None, repr=False)

    def get(self, name: str) -> str:
        return self.attrs.get(name, "")

    @property
    def classes(self) -> frozenset[str]:
        return frozenset(self.get("class").split())

    @property
    def hidden(self) -> bool:
        return self.tag in DROPPED_ELEMENTS or is_hidden(dict(self.attrs))

    def iter(self) -> Iterator[Element]:
        """This element and every descendant element, in document order."""
        stack: list[Element] = [self]
        while stack:
            node = stack.pop()
            yield node
            stack.extend(c for c in reversed(node.children) if isinstance(c, Element))

    def find_all(
        self,
        tag: str | None = None,
        *,
        where: Callable[[Element], bool] | None = None,
    ) -> list[Element]:
        return [
            el
            for el in self.iter()
            if el is not self
            and (tag is None or el.tag == tag)
            and (where is None or where(el))
        ]

    def find(
        self,
        tag: str | None = None,
        *,
        where: Callable[[Element], bool] | None = None,
    ) -> Element | None:
        for el in self.iter():
            if (
                el is not self
                and (tag is None or el.tag == tag)
                and (where is None or where(el))
            ):
                return el
        return None

    def text(self) -> str:
        """The visible text of this subtree, rendered like the mail body."""
        return render_visible(self.children)


def render_visible(nodes: list[Element | str]) -> str:
    """Visible text of a run of sibling nodes, block elements as line breaks."""
    chunks: list[str] = []

    def walk(items: list[Element | str]) -> None:
        for item in items:
            if isinstance(item, str):
                chunks.append(item)
                continue
            if item.hidden:
                continue
            block = item.tag in BLOCK_ELEMENTS
            if block:
                chunks.append("\n")
            walk(item.children)
            if block:
                chunks.append("\n")

    walk(nodes)
    return normalize_rendered_text("".join(chunks))


class _TreeBuilder(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.root = Element("#document")
        self._stack: list[Element] = [self.root]

    @property
    def _current(self) -> Element:
        return self._stack[-1]

    def _append(self, tag: str, attrs: list[tuple[str, str | None]]) -> Element:
        el = Element(
            tag,
            {k.lower(): (v or "") for k, v in attrs},
            parent=self._current,
        )
        self._current.children.append(el)
        return el

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        tag = tag.lower()
        if tag in _CLOSES_P and self._current.tag == "p":
            self._stack.pop()
        el = self._append(tag, attrs)
        if tag not in VOID_ELEMENTS:
            self._stack.append(el)

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self._append(tag.lower(), attrs)

    def handle_endtag(self, tag: str) -> None:
        tag = tag.lower()
        for i in range(len(self._stack) - 1, 0, -1):
            if self._stack[i].tag == tag:
                del self._stack[i:]
                return
        # An end tag with no open element: malformed mail, ignore it.

    def handle_data(self, data: str) -> None:
        self._current.children.append(data)


def parse_html(html: str) -> Element:
    """Parse ``html`` into a tree; malformed input yields what could be recovered."""
    builder = _TreeBuilder()
    try:
        builder.feed(html)
        builder.close()
    except Exception:
        # Hostile, malformed input: keep whatever parsed rather than failing the scan.
        pass
    return builder.root


_WS = re.compile(r"\s+")


def squash(text: str) -> str:
    """Collapse all whitespace, line breaks included, to single spaces."""
    return _WS.sub(" ", text).strip()
