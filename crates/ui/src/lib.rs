pub mod app;
pub mod assets;
pub mod card;
pub mod launcher;
pub mod player;
pub mod theme;
pub mod top_bar;

pub use app::launch_ui;

#[cfg(test)]
mod tests;
