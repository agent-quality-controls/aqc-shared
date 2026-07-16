use std::io::Write;

pub(crate) fn stdout(message: &str) -> std::io::Result<()> {
    std::io::stdout().lock().write_all(message.as_bytes())
}

pub(crate) fn stderr(message: &str) -> std::io::Result<()> {
    std::io::stderr().lock().write_all(message.as_bytes())
}
