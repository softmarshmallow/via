//! Documents.
//!
//! Responsibility: reading every file of a world as TOML with the source span of each value,
//! checking closed keys against the vocabulary, and merging the world file, its layers, its
//! includes and the requested changes into the one merged world the graph is made from. Spans
//! are kept because every diagnostic points back to a file and line.
//!
//! Governed by `spec/documents.md`, with `spec/vocabulary.md` for the words.
