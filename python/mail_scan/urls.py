"""Listing URLs: parsing, token hygiene and admissibility (spec §5.1, §6.3).

Three rules, one parser:

1. **No per-user tokens leave the sidecar.** Alert-mail links carry the recipient's
   identity (Jobindex ``uid``, LinkedIn ``midToken``/``otpToken``/``eid``, Indeed
   ``tk``/``alid``) plus campaign tracking. A stored or displayed listing URL with
   those in it leaks the account to anyone the row is shared with, and makes the same
   job look like a new URL in every mail. ``clean_url`` strips them; board links are
   reduced to the one parameter that names the job.
2. **One job, one key.** ``canonical_url`` (used for the ``url:`` strong key) is the
   clean URL with host and path normalized as well.
3. **No internal addresses.** The sidecar never fetches anything — that is Rust's
   job, behind ``fetch_untrusted``. But an extractor that emits
   ``http://169.254.169.254/...`` has already put an attacker-chosen internal address
   into a row a human might click, so ``is_public_http_url`` filters it here too, as
   defence in depth. Rust's guard is authoritative and re-validates after redirects.
"""

from __future__ import annotations

import ipaddress
import re
from dataclasses import dataclass, replace

# `urllib` is banned in the sidecar (ADR 0004) so that no code path can reach
# `urllib.request` by accident. `urlsplit` would be pure parsing, but the ban is
# deliberately blunt, so URLs are split by hand below.

_ALLOWED_SCHEMES = frozenset({"http", "https"})

# Query parameters that identify the recipient or the campaign, never the job.
# Compared case-insensitively.
_TRACKING_PARAMS = frozenset(
    {
        # Jobindex
        "uid",
        "abtestid",
        "ttid",
        # LinkedIn
        "midtoken",
        "midsig",
        "otptoken",
        "eid",
        "trackingid",
        "refid",
        "lipi",
        # Indeed
        "tk",
        "alid",
        "bb",
        "tmtk",
        "xkcb",
        "rjptk",
        "qd",
        "rd",
        "from",
        "vjk",
        # generic ad-click / newsletter / session ids
        "gclid",
        "fbclid",
        "msclkid",
        "sid",
        "token",
    }
)
_TRACKING_PREFIXES = ("utm_", "trk", "mc_")

_LINKEDIN_JOB = re.compile(r"^/(?:comm/)?jobs/view/(\d+)(?:/.*)?$")
_INDEED_JOB_PATHS = frozenset({"/rc/clk", "/rc/clk/dl", "/viewjob"})
_JOBINDEX_HOSTS = ("jobindex.dk", "it-jobbank.dk")


@dataclass(frozen=True)
class UrlParts:
    """A URL split into the fields the sidecar needs. The fragment is dropped."""

    scheme: str  # lowercased; "" when the URL had none
    host: str  # lowercased, userinfo dropped; an IPv6 literal keeps its brackets
    port: str | None
    path: str
    query: tuple[tuple[str, str], ...]

    @property
    def bare_host(self) -> str:
        """Host without IPv6 brackets or a leading ``www.``, for matching."""
        host = self.host.strip("[]")
        return host[4:] if host.startswith("www.") else host

    def param(self, name: str) -> str | None:
        return next((v for k, v in self.query if k.lower() == name and v), None)

    def unsplit(self, *, default_scheme: str = "https") -> str:
        netloc = self.host if self.port is None else f"{self.host}:{self.port}"
        base = f"{self.scheme or default_scheme}://{netloc}{self.path}"
        if not self.query:
            return base
        return base + "?" + "&".join(f"{k}={v}" if v else k for k, v in self.query)


def _parse_query(query: str) -> tuple[tuple[str, str], ...]:
    pairs: list[tuple[str, str]] = []
    for part in query.split("&"):
        if part:
            key, _, value = part.partition("=")
            pairs.append((key, value))
    return tuple(pairs)


def split_url(url: str) -> UrlParts:
    """Split ``url`` by hand; a missing scheme yields ``scheme == ""``."""
    raw = url.strip()
    scheme, sep, rest = raw.partition("://")
    if not sep or not re.fullmatch(r"[A-Za-z][A-Za-z0-9+.-]*", scheme):
        scheme, rest = "", raw
    rest = rest.partition("#")[0]

    end = next((i for i, ch in enumerate(rest) if ch in "/?"), len(rest))
    authority, remainder = rest[:end], rest[end:]
    path, _, query = remainder.partition("?")

    # Userinfo may itself contain '@'; the host is after the last one.
    hostport = authority.rpartition("@")[2]
    port: str | None = None
    if hostport.startswith("["):
        close = hostport.find("]")
        if close == -1:
            host = hostport
        else:
            host, tail = hostport[: close + 1], hostport[close + 1 :]
            if tail.startswith(":") and tail[1:].isdigit():
                port = tail[1:]
    else:
        host, _, port_s = hostport.partition(":")
        port = port_s if port_s.isdigit() else None

    return UrlParts(
        scheme=scheme.lower(),
        host=host.lower(),
        port=port,
        path=path,
        query=_parse_query(query),
    )


def _is_tracking(key: str) -> bool:
    lowered = key.lower()
    return lowered in _TRACKING_PARAMS or lowered.startswith(_TRACKING_PREFIXES)


def _job_board_form(parts: UrlParts) -> UrlParts | None:
    """The one-parameter form of a known board's job link, if this is one."""
    host = parts.bare_host
    if host == "linkedin.com" or host.endswith(".linkedin.com"):
        m = _LINKEDIN_JOB.match(parts.path)
        if m:
            return replace(parts, path=f"/jobs/view/{m.group(1)}", query=())
    if ".indeed." in f".{host}" and parts.path.rstrip("/") in _INDEED_JOB_PATHS:
        jk = parts.param("jk")
        if jk:
            return replace(parts, path="/viewjob", query=(("jk", jk),))
    if host in _JOBINDEX_HOSTS and parts.path.rstrip("/") == "/c":
        t = parts.param("t")
        if t:
            return replace(parts, path="/c", query=(("t", t),))
    return None


def _without_tokens(parts: UrlParts) -> UrlParts:
    board = _job_board_form(parts)
    if board is not None:
        return board
    return replace(
        parts, query=tuple((k, v) for k, v in parts.query if not _is_tracking(k))
    )


def clean_url(url: str) -> str:
    """``url`` without per-user tokens or tracking; otherwise as the mail gave it.

    This is the URL a listing is emitted with: it still opens the ad, and it can be
    stored, shown and shared without identifying the recipient.
    """
    if not url.strip():
        return ""
    return _without_tokens(split_url(url)).unsplit()


def canonical_url(url: str) -> str:
    """The cleaned URL, normalized so one job has one ``url:`` strong key."""
    parts = _without_tokens(split_url(url))
    host = parts.host[4:] if parts.host.startswith("www.") else parts.host
    path = (
        parts.path[:-1]
        if len(parts.path) > 1 and parts.path.endswith("/")
        else parts.path
    )
    return replace(parts, host=host, path=path).unsplit()


def _is_forbidden_ip(ip: ipaddress.IPv4Address | ipaddress.IPv6Address) -> bool:
    if ip.is_loopback or ip.is_private or ip.is_link_local:
        return True
    if ip.is_multicast or ip.is_unspecified or ip.is_reserved:
        return True
    # CGNAT 100.64.0.0/10 — not covered by is_private.
    if isinstance(ip, ipaddress.IPv4Address):
        octets = ip.packed
        if octets[0] == 100 and 64 <= octets[1] <= 127:
            return True
    return False


def is_public_http_url(url: str) -> bool:
    """True when `url` is an http(s) URL that does not name an internal address.

    Hostnames are not resolved here: DNS from inside the sidecar would be both a
    network call we have forbidden and a rebinding hazard. A literal IP is checked,
    and a name is left for Rust to resolve and re-check. Userinfo is discarded and
    an IPv6 literal unbracketed first, so ``http://user@[::1]:80/x`` is checked as
    ``::1`` rather than something that looks like a hostname.
    """
    parts = split_url(url)
    if parts.scheme not in _ALLOWED_SCHEMES:
        return False
    if parts.host.startswith("[") and not parts.host.endswith("]"):
        return False
    host = parts.host.strip("[]")
    if not host:
        return False

    if host == "localhost" or host.endswith(".localhost"):
        return False

    try:
        return not _is_forbidden_ip(ipaddress.ip_address(host))
    except ValueError:
        # A hostname, not a literal address. Rust resolves and re-checks it.
        return True
