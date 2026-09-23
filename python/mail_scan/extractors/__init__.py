"""Extractor package."""

from __future__ import annotations

from mail_scan.extractors.base import extract_listings
from mail_scan.extractors.indeed import extract_indeed
from mail_scan.extractors.jobindex import extract_jobindex
from mail_scan.extractors.linkedin import extract_linkedin
from mail_scan.extractors.types import ExtractedListing

__all__ = [
    "ExtractedListing",
    "extract_indeed",
    "extract_jobindex",
    "extract_linkedin",
    "extract_listings",
]
