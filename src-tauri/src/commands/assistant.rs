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
/// ⚠️ Bağlamı ÖN YÜZ derliyor: kullanıcının **+ menüsünden seçtiği** kaynakların satırları
/// + rapor özeti (`src/assistantSources.ts`). Kullanıcı kararı buydu — tüm raporu (2.190 EOL
/// satırı dahil) her mesajda göndermek gecikmeyi artırır ve uzun bağlamda model detayı
/// karıştırmaya daha yatkın olur.
///
/// `sources`: yüklü kaynakların okunur adları ("Fırsatlar, Katalog"). Prompt'a yazılıyor ki
/// model **"seçili değil"** ile **"veride yok"**u ayırt edebilsin.
#[tauri::command]
pub async fn assistant_ask(
    state: State<'_, AppState>,
    history: Vec<gemini::ChatMessage>,
    context: String,
    sources: String,
    on_event: Channel<AssistantEvent>,
) -> Result<String, String> {
    let (key, zincir) = {
        let conn = state.conn.lock().unwrap();
        (
            db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default(),
            sohbet_zinciri(&conn),
        )
    };

    let system = gemini::assistant_system_prompt(&context, &sources);
    let toplayici = CallToplayici::default();
    let kanal = toplayici.kanal();
    let chain = gemini::ChainCtx { models: zincir, log: Some(&kanal) };
    let sonuc = gemini::chat_stream(&key, &system, &history, &chain, |e| {
        // Kanal hatası (pencere kapandı vb.) sohbeti düşürmesin: kullanıcı zaten gitmiş.
        let _ = match e {
            gemini::ChatEvent::Thinking => on_event.send(AssistantEvent::Thinking),
            gemini::ChatEvent::Chunk(t) => on_event.send(AssistantEvent::Chunk { text: t.to_string() }),
        };
    })
    .await;

    {
        let conn = state.conn.lock().unwrap();
        toplayici.yaz(&conn, "chat");
    }
    Ok(sonuc?.model)
}

/// Kaydedilen sohbetin liste görünümü — mesaj gövdeleri taşınmaz, yalnızca üstveri.
#[derive(Serialize)]
pub struct ChatSessionMeta {
    pub id: i64,
    pub title: String,
    /// Sohbetin bağlam kaynakları — **virgülle ayrılmış anahtar listesi**
    /// ("opportunities,catalog"). Boş olabilir.
    ///
    /// ⚠️ Adı tarihsel: v0.11.0'a (Faz A) kadar tek bir ekran anahtarı tutuyordu. Şema
    /// değiştirilmedi, çünkü tek değerli eski kayıtlar tek elemanlı liste olarak
    /// okunuyor — yeni sütun açmak göç ve yedekleme yüzeyini büyütürdü.
    pub tool_page: String,
    pub messages: i64,
    pub model: String,
    pub updated_at: String,
}

/// Saklanan en fazla sohbet sayısı.
///
/// ⚠️ Kullanıcı kararı silme YETKİSİNİN kendisinde olması; bu sınır o yetkinin yerine geçmiyor,
/// yalnızca tablonun sessizce sınırsız büyümesini engelliyor. 50 sohbet birkaç yüz KB — yedek
/// dosyasında fark ettirmez. Sürüm geçmişindeki `history::MAX = 5` burada çok dar kalırdı:
/// orada aynı alanın eski hâlleri tutuluyor, burada ayrı ayrı konuşmalar.
const MAX_SESSIONS: usize = 50;

/// Sohbetleri en yeni başta listeler.
#[tauri::command]
pub fn list_chat_sessions(state: State<'_, AppState>) -> Result<Vec<ChatSessionMeta>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, title, COALESCE(tool_page,''), messages_json, COALESCE(model,''), updated_at
             FROM chat_sessions ORDER BY updated_at DESC",
        )
        .map_err(|e| format!("Sohbetler okunamadı: {e}"))?;
    let rows = stmt
        .query_map([], |r| {
            let json: String = r.get(3)?;
            Ok(ChatSessionMeta {
                id: r.get(0)?,
                title: r.get(1)?,
                tool_page: r.get(2)?,
                // Mesaj sayısı listede gösteriliyor; gövdeleri ön yüze taşımaya gerek yok.
                messages: serde_json::from_str::<Vec<gemini::ChatMessage>>(&json)
                    .map(|v| v.len() as i64)
                    .unwrap_or(0),
                model: r.get(4)?,
                updated_at: r.get(5)?,
            })
        })
        .map_err(|e| format!("Sohbetler okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Bir sohbetin mesajlarını getirir.
#[tauri::command]
pub fn get_chat_session(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Vec<gemini::ChatMessage>, String> {
    let conn = state.conn.lock().unwrap();
    let json: String = conn
        .query_row("SELECT messages_json FROM chat_sessions WHERE id = ?1", [id], |r| r.get(0))
        .map_err(|_| "Bu sohbet artık yok.".to_string())?;
    // Bozuk JSON'da panik atma: geçmiş yardımcı bir özellik (bkz. core/src/history.rs).
    Ok(serde_json::from_str(&json).unwrap_or_default())
}

/// Sohbeti kaydeder/günceller ve kimliğini döndürür.
///
/// `id` yoksa yeni kayıt açılır — başlık ilk kullanıcı sorusundan türetilir.
#[tauri::command]
pub fn save_chat_session(
    state: State<'_, AppState>,
    id: Option<i64>,
    messages: Vec<gemini::ChatMessage>,
    // Virgüllü kaynak listesi — bkz. `ChatSessionMeta::tool_page`.
    tool_page: String,
    model: String,
) -> Result<i64, String> {
    if messages.is_empty() {
        return Err("Kaydedilecek mesaj yok.".into());
    }
    let json = serde_json::to_string(&messages)
        .map_err(|e| format!("Sohbet serialize edilemedi: {e}"))?;
    let now = now_str();
    let conn = state.conn.lock().unwrap();

    if let Some(id) = id {
        let n = conn
            .execute(
                // ⚠️ `tool_page` de güncelleniyor: kullanıcı kaynak seçimini sohbetin
                // ortasında değiştirebiliyor (Faz A) ve sohbet yeniden açıldığında
                // SON seçim geri gelmeli.
                "UPDATE chat_sessions SET messages_json = ?2, model = ?3, updated_at = ?4,
                        tool_page = ?5
                 WHERE id = ?1",
                params![id, json, model, now, tool_page],
            )
            .map_err(|e| format!("Sohbet kaydedilemedi: {e}"))?;
        // Kullanıcı sohbeti başka bir yerden silmişse güncelleme 0 satır etkiler → yeni kayıt aç.
        if n > 0 {
            return Ok(id);
        }
    }

    let title = messages
        .iter()
        .find(|m| m.role == "user")
        .map(|m| gemini::session_title(&m.text))
        .unwrap_or_else(|| gemini::session_title(""));
    conn.execute(
        "INSERT INTO chat_sessions (title, tool_page, messages_json, model, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![title, tool_page, json, model, now],
    )
    .map_err(|e| format!("Sohbet kaydedilemedi: {e}"))?;
    let new_id = conn.last_insert_rowid();

    // Sınırı aşan en eski sohbetleri düşür.
    let _ = conn.execute(
        "DELETE FROM chat_sessions WHERE id NOT IN
           (SELECT id FROM chat_sessions ORDER BY updated_at DESC LIMIT ?1)",
        params![MAX_SESSIONS as i64],
    );
    Ok(new_id)
}

/// Tek bir sohbeti siler. **Kullanıcı eylemi** — otomatik temizlik değil.
#[tauri::command]
pub fn delete_chat_session(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute("DELETE FROM chat_sessions WHERE id = ?1", [id])
        .map_err(|e| format!("Sohbet silinemedi: {e}"))?;
    Ok(())
}

/// Tüm sohbet geçmişini siler. Arayüz bunu açık onayla çağırıyor.
#[tauri::command]
pub fn delete_all_chat_sessions(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute("DELETE FROM chat_sessions", [])
        .map_err(|e| format!("Sohbetler silinemedi: {e}"))?;
    Ok(())
}
