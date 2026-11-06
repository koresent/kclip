pub mod classifier;
pub mod code;
pub mod color;
pub mod files;
pub mod url;

pub use classifier::{ClassificationResult, classify_content};
pub use code::{DetectedCode, detect_code};
pub use color::{DetectedColor, detect_color};
pub use files::{DetectedFileList, detect_file_list};
pub use url::{DetectedUrl, detect_url};
