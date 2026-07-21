"""Index-free grep-based retrieval primitives for AI agent tool loops.

The implementation lives in a native extension built from the grepsearch
Rust crate. This package re-exports the public classes.
"""

from grepsearch._native import (
    DirEntry,
    FileHit,
    FileMatches,
    FileSlice,
    GrepResult,
    LineMatch,
    SearchEngine,
    __version__,
)

__all__ = [
    "DirEntry",
    "FileHit",
    "FileMatches",
    "FileSlice",
    "GrepResult",
    "LineMatch",
    "SearchEngine",
    "__version__",
]
