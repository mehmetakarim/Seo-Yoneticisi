//! # seo-core — uygulamanın Tauri'den bağımsız çekirdeği
//!
//! Feed ayrıştırma, veritabanı, Gemini üretimi, SEO araştırması (Ahrefs/GSC), görsel
//! değerlendirme, IdeaSoft istemcisi ve doğrulama kuralları burada. **Hiçbiri `tauri`
//! crate'ine bağlı değil.**
//!
//! Neden ayrı crate: bu kod 4.594 satır ve testlerin 81'i burada. Tek crate'teyken bir
//! testi çalıştırmak için Tauri'nin tüm yığını (582 bağımlılığın ~102'si; objc2, wry,
//! webkit bağlamaları) derleniyordu. Ayrı crate olunca `cargo test -p seo-core` bunların
//! hiçbirini derlemiyor.
//!
//! Tauri katmanı (`commands.rs`, `lib.rs`) bu crate'i kullanır; ters yönde bağımlılık
//! YOKTUR ve olmamalıdır — bir çekirdek modülü `AppState`'e veya `tauri::` bir şeye
//! ihtiyaç duyuyorsa, o ihtiyaç Tauri katmanında çözülmelidir.

pub mod db;
pub mod feed;
pub mod fingerprint;
pub mod gemini;
pub mod history;
pub mod ideasoft;
pub mod jsonld;
pub mod metrics;
pub mod images;
pub mod opportunity;
pub mod seo_data;
pub mod sync;
pub mod validation;
