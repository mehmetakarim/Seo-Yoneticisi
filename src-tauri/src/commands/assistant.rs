//! Yapay Zekâ Asistanı komutu — akışlı sohbet.
//!
//! ⚠️ **Asistan HİÇBİR ŞEY YAZMAZ.** Ne canonical, ne meta, ne IdeaSoft gönderimi. Bunların
//! hepsi kendi açık onaylı akışlarında kalıyor. Kullanıcının "toplu değil, gerektiğinde ve
//! tek tek, onayla" kuralı bir sohbet arayüzüyle delinmez — asistan yalnızca okur ve yorumlar.
//!
//! ⚠️ **Tespit ölçümdür, asistan yorum katmanıdır.** Hangi sayfa kaçıncı sırada, kaç tıklama
//! kaçıyor — bunlar GSC'den gelen ölçümler. Asistanın işi bu sayıları uydurmak değil,
//! verilenleri açıklamak ve önceliklendirmek.

use super::*;
use tauri::ipc::Channel;

/// Ön yüze akan olay. `kind` ayrımı bilinçli: arayüz "düşünüyor…" göstergesini
/// gerçek bir sinyale dayandırıyor, kör bir zamanlayıcıya değil.
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AssistantEvent {
    Thinking,
    Chunk { text: String },
}

/// Sohbet turu. Yanıt `on_event` kanalından parça parça akar; komut biterken kullanılan
/// modeli döndürür (arayüz rozetinde gösteriliyor — hangi modelin cevapladığı görünsün).
///
/// ⚠️ Bağlamı ÖN YÜZ derliyor: kullanıcının o an baktığı ekranın satırları + rapor özeti.
/// Kullanıcı kararı buydu — tüm raporu (2.190 EOL satırı dahil) her mesajda göndermek
/// gecikmeyi artırır ve uzun bağlamda model detayı karıştırmaya daha yatkın olur.
#[tauri::command]
pub async fn assistant_ask(
    state: State<'_, AppState>,
    history: Vec<gemini::ChatMessage>,
    context: String,
    on_event: Channel<AssistantEvent>,
) -> Result<String, String> {
    let key = {
        let conn = state.conn.lock().unwrap();
        db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default()
    };

    let produced = gemini::chat_stream(&key, &gemini::assistant_system_prompt(&context), &history, |e| {
        // Kanal hatası (pencere kapandı vb.) sohbeti düşürmesin: kullanıcı zaten gitmiş.
        let _ = match e {
            gemini::ChatEvent::Thinking => on_event.send(AssistantEvent::Thinking),
            gemini::ChatEvent::Chunk(t) => on_event.send(AssistantEvent::Chunk { text: t.to_string() }),
        };
    })
    .await?;

    Ok(produced.model.to_string())
}
