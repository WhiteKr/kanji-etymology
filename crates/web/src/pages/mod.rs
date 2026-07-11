//! 페이지 컴포넌트 모음 (라우트 1개 = 파일 1개).

mod kanji;
mod landing;
mod not_found;
mod radical;
mod search;

pub use kanji::KanjiPage;
pub use landing::Landing;
pub use not_found::NotFound;
pub use radical::RadicalPage;
pub use search::SearchPage;
