"""URL admissibility for extracted listings (spec §6.3).

The sidecar never fetches anything — that is Rust's job, behind `fetch_untrusted`.
But an extractor that happily emits `http://169.254.169.254/...` as a listing URL has
already put an attacker-chosen internal address into a row a human might click, and
into a field the enrichment path will later be asked to fetch.

So the filter lives here too, as defence in depth. Rust's guard is authoritative and
re-validates after every redirect; this one just stops the obviously-hostile from
becoming a listing in the first place.
"""

from __future__ import annotations

import ipaddress

# `urllib` is banned in the sidecar (ADR 0004) so that no code path can reach
# `urllib.request` by accident. `urlsplit` would be pure parsing, but the ban is
# deliberately blunt, so the few fields we need are parsed by hand below.

_ALLOWED_SCHEMES = frozenset({"http", "https"})
_AUTHORITY_END = "/?#"


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


def _split_scheme_and_host(url: str) -> tuple[str, str]:
    """Return `(scheme, host)` lowercased, or `("", "")` when unparseable.

    Only the two fields the admissibility check needs. Userinfo is discarded and an
    IPv6 literal is unbracketed, so `http://user@[::1]:80/x` yields `("http", "::1")`
    rather than something that looks like a hostname.
    """
    scheme, sep, rest = url.strip().partition("://")
    if not sep:
        return "", ""

    authority = rest
    for i, ch in enumerate(rest):
        if ch in _AUTHORITY_END:
            authority = rest[:i]
            break

    # Userinfo may itself contain '@'; the host is after the last one.
    _, _, hostport = authority.rpartition("@")

    if hostport.startswith("["):
        end = hostport.find("]")
        if end == -1:
            return "", ""
        return scheme.lower(), hostport[1:end].lower()

    host, _, _ = hostport.partition(":")
    return scheme.lower(), host.lower()


def is_public_http_url(url: str) -> bool:
    """True when `url` is an http(s) URL that does not name an internal address.

    Hostnames are not resolved here: DNS from inside the sidecar would be both a
    network call we have forbidden and a rebinding hazard. A literal IP is checked,
    and a name is left for Rust to resolve and re-check.
    """
    scheme, host = _split_scheme_and_host(url)
    if scheme not in _ALLOWED_SCHEMES or not host:
        return False

    if host == "localhost" or host.endswith(".localhost"):
        return False

    try:
        return not _is_forbidden_ip(ipaddress.ip_address(host))
    except ValueError:
        # A hostname, not a literal address. Rust resolves and re-checks it.
        return True
