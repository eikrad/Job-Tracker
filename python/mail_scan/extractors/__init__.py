"""Extractor package."""

from __future__ import annotations

from mail_scan.extractors.base import extract_listings
from mail_scan.extractors.generic import extract_generic
from mail_scan.extractors.indeed import extract_indeed
from mail_scan.extractors.types import ExtractedListing

__all__ = [
    "ExtractedListing",
    "extract_generic",
    "extract_indeed",
    "extract_listings",
]
