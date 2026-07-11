//! 페이지 컴포넌트 모음 (라우트 1개 = 파일 1개).

mod about;
mod browse;
mod kanji;
mod landing;
mod not_found;
mod radical;
mod radicals;
mod search;

pub use about::AboutPage;
pub use browse::BrowsePage;
pub use kanji::KanjiPage;
pub use landing::Landing;
pub use not_found::NotFound;
pub use radical::RadicalPage;
pub use radicals::RadicalsPage;
pub use search::SearchPage;
