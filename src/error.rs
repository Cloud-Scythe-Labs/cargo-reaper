use std::{fmt, ops, process};

use codespan_reporting::{
    diagnostic, files,
    term::{self, termcolor},
};

/// An identifier that corresponds to some [`codespan_reporting::files::SimpleFile`].
pub(crate) type FileId = usize;

/// The message to display for some diagnostic error.
pub(crate) type Message = String;

/// Collection of diagnostics for toml files that is context aware.
#[derive(Default)]
pub(crate) struct TomlErrorEmitter<FilePath, FileContents>
where
    FilePath: fmt::Display + Clone + Default + Sized,
    FileContents: AsRef<str> + Clone + Default,
{
    /// A collection of file paths and their contents.
    db: files::SimpleFiles<FilePath, FileContents>,

    /// A collection of diagnostic data containing identifiers corresponding to the db.
    errors: Vec<diagnostic::Diagnostic<FileId>>,
}
impl<FilePath, FileContents> TomlErrorEmitter<FilePath, FileContents>
where
    FilePath: fmt::Display + Clone + Default + Sized,
    FileContents: AsRef<str> + Clone + Default,
{
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Given some diagnostic error info, insert the file and string contents into the db, and
    /// add an error built from the messages and spans to the list of errors. The file id is
    /// automatically handled between the diagnostic error and the db.
    // TODO: Refactor this so it is more legible
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn insert_err(
        &mut self,
        path: FilePath,
        contents: FileContents,
        message: impl Into<Message>,
        primary_span: ops::Range<usize>,
        primary_msg: Option<impl Into<Message>>,
        secondary_span: Option<ops::Range<usize>>,
        secondary_msg: Option<impl Into<Message>>,
    ) {
        let error = diagnostic::Diagnostic::error().with_message(message.into());
        let mut labels: Vec<diagnostic::Label<usize>> = Vec::with_capacity(2);
        let mut primary_label: diagnostic::Label<usize> = diagnostic::Label::primary(
            self.db.add(path.clone(), contents.clone()),
            primary_span.clone(),
        );
        if let Some(primary_msg) = primary_msg {
            primary_label = primary_label.with_message(primary_msg.into());
        }
        labels.push(primary_label);
        if let Some(secondary_msg) = secondary_msg {
            labels.push(
                diagnostic::Label::secondary(
                    self.db.add(path, contents),
                    secondary_span.unwrap_or(primary_span),
                )
                .with_message(secondary_msg.into()),
            );
        }
        self.errors.push(error.with_labels(labels))
    }

    /// Exit with errors, if any.
    pub(crate) fn emit(self) -> anyhow::Result<()> {
        if !self.errors.is_empty() {
            for error in self.errors.iter().rev() {
                term::emit(
                    &mut termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto),
                    &Default::default(),
                    &self.db,
                    error,
                )?;
            }
            process::exit(1);
        }
        Ok(())
    }
}
