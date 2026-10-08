use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt::Write as _;
use std::fs::File;
use std::io::BufReader;

use serde::Deserialize;

// ─────────────────────────────────────────────────────────────────────────────
// Корень экспорта
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RawExport {
    pub id: Option<i64>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type", default)]
    pub typ: Option<String>,
    #[serde(default)]
    pub messages: Vec<RawMessage>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Сырое сообщение
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct RawMessage {
    pub id: i64,

    #[serde(default)]
    pub message_id: Option<i64>,

    #[serde(rename = "type", default)]
    pub typ: String,

    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub date_unixtime: Option<String>,

    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub from_id: Option<String>,

    #[serde(default)]
    pub reply_to_message_id: Option<i64>,

    #[serde(default)]
    pub edited: Option<String>,
    #[serde(default)]
    pub edited_unixtime: Option<String>,
    #[serde(default)]
    pub discard_reason: Option<String>,

    #[serde(default)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    pub sticker_emoji: Option<String>,
    #[serde(default)]
    pub photo: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub width: Option<i64>,
    #[serde(default)]
    pub height: Option<i64>,

    #[serde(default, deserialize_with = "de_text")]
    pub text: String,
    #[serde(default)]
    pub text_entities: Vec<TextEntity>,

    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub actor_id: Option<String>,
    #[serde(default)]
    pub action: Option<String>,

    #[serde(default)]
    pub forwarded_from: Option<String>,
    #[serde(default)]
    pub saved_from: Option<String>,
    #[serde(default)]
    pub via_bot: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Текстовые сущности
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct TextEntity {
    #[serde(rename = "type")]
    pub typ: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub href: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// text: String | [String | {text: String, ...}]
// ─────────────────────────────────────────────────────────────────────────────

fn de_text<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s,
        serde_json::Value::Array(parts) => {
            let mut out = String::new();
            for p in parts {
                match p {
                    serde_json::Value::String(s) => out.push_str(&s),
                    serde_json::Value::Object(o) => {
                        if let Some(s) = o.get("text").and_then(|t| t.as_str()) {
                            out.push_str(s);
                        }
                    }
                    _ => {}
                }
            }
            out
        }
        other => return Err(D::Error::custom(format!("unexpected text type: {other}"))),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Общие поля
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct Common {
    pub id: i64,
    pub message_id: Option<i64>,
    pub date_unixtime: String,
    pub from: Option<String>,
    pub from_id: Option<String>,
    pub reply_to_message_id: Option<i64>,
    pub edited: Option<String>,
    pub discard_reason: Option<String>,
    pub text: String,
}

impl Common {
    fn from_raw(r: &RawMessage) -> Self {
        Self {
            id: r.id,
            message_id: r.message_id,
            date_unixtime: r.date_unixtime.clone().unwrap_or_default(),
            from: r.from.clone(),
            from_id: r.from_id.clone(),
            reply_to_message_id: r.reply_to_message_id,
            edited: r.edited.clone(),
            discard_reason: r.discard_reason.clone(),
            text: r.text.clone(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Типизированные сообщения
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum Message {
    Text(TextMessage),
    Photo(PhotoMessage),
    Video(VideoMessage),
    Sticker(StickerMessage),
    File(FileMessage),
    Service(ServiceMessage),
    Unknown(RawMessage),
}

impl Message {
    pub fn common(&self) -> Option<&Common> {
        match self {
            Message::Text(m) => Some(&m.common),
            Message::Photo(m) => Some(&m.common),
            Message::Video(m) => Some(&m.common),
            Message::Sticker(m) => Some(&m.common),
            Message::File(m) => Some(&m.common),
            Message::Service(m) => Some(&m.common),
            Message::Unknown(_) => None,
        }
    }

    pub fn sender(&self) -> (Option<&str>, Option<&str>) {
        match self {
            Message::Text(m) => (m.common.from_id.as_deref(), m.common.from.as_deref()),
            Message::Photo(m) => (m.common.from_id.as_deref(), m.common.from.as_deref()),
            Message::Video(m) => (m.common.from_id.as_deref(), m.common.from.as_deref()),
            Message::Sticker(m) => (m.common.from_id.as_deref(), m.common.from.as_deref()),
            Message::File(m) => (m.common.from_id.as_deref(), m.common.from.as_deref()),
            Message::Service(m) => (m.common.from_id.as_deref(), m.common.from.as_deref()),
            Message::Unknown(r) => (r.from_id.as_deref(), r.from.as_deref()),
        }
    }

    pub fn from_raw(raw: RawMessage) -> Self {
        match raw.typ.as_str() {
            "service" => return Message::Service(ServiceMessage::from_raw(raw)),
            "message" => {}
            _ => return Message::Unknown(raw),
        }

        match raw.media_type.as_deref() {
            Some("sticker") => Message::Sticker(StickerMessage::from_raw(raw)),
            Some("video_file") => Message::Video(VideoMessage::from_raw(raw)),
            Some("voice_message") | Some("audio_file") => Message::File(FileMessage::from_raw(raw)),
            Some("animation") => Message::Video(VideoMessage::from_raw(raw)),
            _ if raw.photo.is_some() => Message::Photo(PhotoMessage::from_raw(raw)),
            _ if raw.file.is_some() => Message::File(FileMessage::from_raw(raw)),
            _ => Message::Text(TextMessage::from_raw(raw)),
        }
    }
}

#[derive(Debug)]
pub struct TextMessage {
    pub common: Common,
}

#[derive(Debug)]
pub struct PhotoMessage {
    pub common: Common,
    pub photo: Option<String>,
}

#[derive(Debug)]
pub struct VideoMessage {
    pub common: Common,
    pub file: Option<String>,
    pub mime_type: Option<String>,
    pub duration_seconds: Option<i64>,
}

#[derive(Debug)]
pub struct StickerMessage {
    pub common: Common,
    pub file: Option<String>,
    pub sticker_emoji: Option<String>,
}

#[derive(Debug)]
pub struct FileMessage {
    pub common: Common,
    pub file: Option<String>,
    pub mime_type: Option<String>,
    pub duration_seconds: Option<i64>,
    pub media_type: Option<String>,
}

#[derive(Debug)]
pub struct ServiceMessage {
    pub common: Common,
    pub actor: Option<String>,
    pub action: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Помощники
// ─────────────────────────────────────────────────────────────────────────────

fn is_placeholder(s: &str) -> bool {
    s.starts_with("(File not included")
}

fn clean_file(s: Option<String>) -> Option<String> {
    match s {
        Some(v) if is_placeholder(&v) => None,
        Some(v) if v.is_empty() => None,
        other => other,
    }
}

impl TextMessage {
    fn from_raw(r: RawMessage) -> Self {
        Self {
            common: Common::from_raw(&r),
        }
    }
}

impl PhotoMessage {
    fn from_raw(r: RawMessage) -> Self {
        Self {
            common: Common::from_raw(&r),
            photo: clean_file(r.photo.clone()),
        }
    }
}

impl VideoMessage {
    fn from_raw(r: RawMessage) -> Self {
        Self {
            common: Common::from_raw(&r),
            file: clean_file(r.file.clone()),
            mime_type: r.mime_type.clone(),
            duration_seconds: r.duration_seconds,
        }
    }
}

impl StickerMessage {
    fn from_raw(r: RawMessage) -> Self {
        Self {
            common: Common::from_raw(&r),
            file: clean_file(r.file.clone()),
            sticker_emoji: r.sticker_emoji.clone(),
        }
    }
}

impl FileMessage {
    fn from_raw(r: RawMessage) -> Self {
        Self {
            common: Common::from_raw(&r),
            file: clean_file(r.file.clone()),
            mime_type: r.mime_type.clone(),
            duration_seconds: r.duration_seconds,
            media_type: r.media_type.clone(),
        }
    }
}

impl ServiceMessage {
    fn from_raw(r: RawMessage) -> Self {
        Self {
            common: Common::from_raw(&r),
            actor: r.actor.clone(),
            action: r.action.clone(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Экранирование, base36, kv-помощники
// ─────────────────────────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}

fn kv<V: AsRef<str>>(key: &str, val: Option<V>) -> Option<String> {
    val.and_then(|v| {
        let s = v.as_ref();
        if s.is_empty() {
            None
        } else {
            Some(format!("{}={}", key, esc(s)))
        }
    })
}

fn kv_num(key: &str, val: Option<i64>) -> Option<String> {
    val.and_then(|v| Some(format!("{key}={v}")))
}

// ─────────────────────────────────────────────────────────────────────────────
// Карты сокращений (для шапки)
// ─────────────────────────────────────────────────────────────────────────────

/// Код типа сообщения → полное имя. Порядок = порядок в шапке.
const TYPE_CODES: &[(&str, &str)] = &[
    ("t", "text"),
    ("p", "photo"),
    ("v", "video"),
    ("s", "sticker"),
    ("f", "file"),
    ("sv", "service"),
    ("u", "unknown"),
];

/// Код ключа extras → полное имя. `ed` — флаг без значения.
const KEY_CODES: &[(&str, &str)] = &[
    ("r", "reply"),
    ("ed", "edited (flag, no value)"),
    ("d", "discard_reason"),
    ("k", "kind (media_type for file)"),
    ("m", "mime_type"),
    ("du", "duration_seconds"),
    ("fl", "file"),
    ("e", "emoji"),
    ("a", "action"),
    ("ac", "actor"),
    ("mt", "media_type (unknown)"),
];

// ─────────────────────────────────────────────────────────────────────────────
// Сборка карты отправителей
// ─────────────────────────────────────────────────────────────────────────────

fn build_senders(messages: &[Message]) -> (Vec<(String, String)>, HashMap<String, usize>) {
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut list: Vec<(String, String)> = Vec::new();

    for m in messages {
        let (fid, fname) = m.sender();
        if let Some(fid) = fid {
            if !fid.is_empty() && !index.contains_key(fid) {
                let idx = list.len();
                index.insert(fid.to_string(), idx);
                list.push((fid.to_string(), fname.unwrap_or("").to_string()));
            }
        }
    }

    (list, index)
}

fn sender_idx(from_id: &Option<String>, index: &HashMap<String, usize>) -> String {
    from_id
        .as_ref()
        .and_then(|fid| index.get(fid).copied())
        .map(|i| i.to_string())
        .unwrap_or_else(|| "_".to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Сборка одной строки
// ─────────────────────────────────────────────────────────────────────────────

fn compact_line(m: &Message, index: &HashMap<String, usize>) -> String {
    let (sid, msg_id, time, media_type, extras, text) = match m {
        Message::Text(t) => {
            let c = &t.common;
            (
                sender_idx(&c.from_id, index),
                c.message_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| c.id.to_string()),
                c.date_unixtime.clone(),
                "t".to_string(),
                build_extras(c, &[]),
                c.text.clone(),
            )
        }

        Message::Photo(p) => {
            let c = &p.common;
            let mut extra: Vec<String> = Vec::new();
            extra.extend(kv("fl", p.photo.as_deref()));
            (
                sender_idx(&c.from_id, index),
                c.message_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| c.id.to_string()),
                c.date_unixtime.clone(),
                "p".to_string(),
                build_extras(c, &extra),
                c.text.clone(),
            )
        }

        Message::Video(v) => {
            let c = &v.common;
            let mut extra: Vec<String> = Vec::new();
            extra.extend(kv_num("du", v.duration_seconds));
            extra.extend(kv("m", v.mime_type.as_deref()));
            extra.extend(kv("fl", v.file.as_deref()));
            (
                sender_idx(&c.from_id, index),
                c.message_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| c.id.to_string()),
                c.date_unixtime.clone(),
                "v".to_string(),
                build_extras(c, &extra),
                c.text.clone(),
            )
        }

        Message::Sticker(s) => {
            let c = &s.common;
            let mut extra: Vec<String> = Vec::new();
            extra.extend(kv("e", s.sticker_emoji.as_deref()));
            extra.extend(kv("fl", s.file.as_deref()));
            (
                sender_idx(&c.from_id, index),
                c.message_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| c.id.to_string()),
                c.date_unixtime.clone(),
                "s".to_string(),
                build_extras(c, &extra),
                c.text.clone(),
            )
        }

        Message::File(f) => {
            let c = &f.common;
            let mut extra: Vec<String> = Vec::new();
            extra.extend(kv("k", f.media_type.as_deref()));
            extra.extend(kv_num("du", f.duration_seconds));
            extra.extend(kv("m", f.mime_type.as_deref()));
            extra.extend(kv("fl", f.file.as_deref()));
            (
                sender_idx(&c.from_id, index),
                c.message_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| c.id.to_string()),
                c.date_unixtime.clone(),
                "f".to_string(),
                build_extras(c, &extra),
                c.text.clone(),
            )
        }

        Message::Service(s) => {
            let c = &s.common;
            let mut extra: Vec<String> = Vec::new();
            extra.extend(kv("a", s.action.as_deref()));
            extra.extend(kv("ac", s.actor.as_deref()));
            (
                sender_idx(&c.from_id, index),
                c.message_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| c.id.to_string()),
                c.date_unixtime.clone(),
                "sv".to_string(),
                build_extras(c, &extra),
                c.text.clone(),
            )
        }

        Message::Unknown(r) => {
            let mut extra: Vec<String> = Vec::new();
            extra.extend(kv("mt", r.media_type.as_deref()));
            extra.extend(kv_num("du", r.duration_seconds));
            extra.extend(kv("m", r.mime_type.as_deref()));
            extra.extend(kv("e", r.sticker_emoji.as_deref()));
            extra.extend(kv("fl", r.file.as_deref()));
            extra.extend(kv("a", r.action.as_deref()));
            (
                sender_idx(&r.from_id, index),
                r.message_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| r.id.to_string()),
                r.date_unixtime.clone().unwrap_or_default(),
                "u".to_string(),
                build_extras_raw(r, extra),
                r.text.clone(),
            )
        }
    };

    let mut out = String::with_capacity(64);
    let _ = write!(out, "{}\t{}\t{}\t{}", sid, msg_id, time, media_type);
    if !extras.is_empty() {
        out.push('\t');
        out.push_str(&extras);
    }
    if !text.is_empty() {
        out.push('\t');
        out.push_str(&esc(&text));
    }
    out
}

fn build_extras(c: &Common, extra: &[String]) -> String {
    let mut parts: Vec<String> = extra.to_vec();
    if let Some(r) = c.reply_to_message_id {
        parts.push(format!("r={r}"));
    }
    if c.edited.is_some() {
        parts.push("ed".to_string());
    }
    if let Some(d) = &c.discard_reason {
        if !d.is_empty() {
            parts.push(format!("d={}", esc(d)));
        }
    }
    parts.join(",")
}

fn build_extras_raw(r: &RawMessage, mut extra: Vec<String>) -> String {
    if let Some(rid) = r.reply_to_message_id {
        extra.push(format!("r={rid}"));
    }
    if r.edited.is_some() {
        extra.push("ed".to_string());
    }
    if let Some(d) = &r.discard_reason {
        if !d.is_empty() {
            extra.push(format!("d={}", esc(d)));
        }
    }
    extra.join(",")
}

// ─────────────────────────────────────────────────────────────────────────────
// Сборка всего дампа: SENDERS + TYPES + KEYS + MESSAGES
// ─────────────────────────────────────────────────────────────────────────────

fn compact_dump(messages: &[Message]) -> String {
    let (senders, index) = build_senders(messages);

    let mut out = String::with_capacity(messages.len() * 60 + 512);

    // ── SENDERS ─────────────────────────────────────────────────────────
    out.push_str("SENDERS\n");
    for (i, (fid, name)) in senders.iter().enumerate() {
        let _ = writeln!(out, "{i}\t{}\t{}", esc(fid), esc(name));
    }

    // ── TYPES ───────────────────────────────────────────────────────────
    out.push_str("TYPES\n");
    for (code, full) in TYPE_CODES {
        let _ = writeln!(out, "{code}\t{full}");
    }

    // ── KEYS ────────────────────────────────────────────────────────────
    out.push_str("KEYS\n");
    for (code, full) in KEY_CODES {
        let _ = writeln!(out, "{code}\t{full}");
    }

    // ── MESSAGES ────────────────────────────────────────────────────────
    out.push_str("MESSAGES\n");
    for m in messages {
        out.push_str(&compact_line(m, &index));
        out.push('\n');
    }

    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Известные ключи (диагностика)
// ─────────────────────────────────────────────────────────────────────────────

const KNOWN_KEYS: &[&str] = &[
    "id",
    "message_id",
    "type",
    "date",
    "date_unixtime",
    "from",
    "from_id",
    "reply_to_message_id",
    "edited",
    "edited_unixtime",
    "discard_reason",
    "media_type",
    "mime_type",
    "duration_seconds",
    "sticker_emoji",
    "photo",
    "file",
    "thumbnail",
    "width",
    "height",
    "text",
    "text_entities",
    "actor",
    "actor_id",
    "action",
    "forwarded_from",
    "saved_from",
    "via_bot",
];

// ─────────────────────────────────────────────────────────────────────────────
// main
// ─────────────────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // program.exe <input> [output="./result.txt"]
    let args: Vec<String> = env::args().collect();

    let in_path = match args.get(1) {
        Some(p) => p.clone(),
        None => {
            eprintln!("Usage: {} <input.json> [output.txt]", args[0]);
            eprintln!("  input   — обязательный, путь к messages.json");
            eprintln!("  output  — опциональный, по умолчанию ./result.txt");
            std::process::exit(1);
        }
    };

    let out_path = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "./result.txt".to_string());

    println!("Input:  {in_path}");
    println!("Output: {out_path}");

    let file = File::open(in_path)?;
    let reader = BufReader::new(file);
    let root: serde_json::Value = serde_json::from_reader(reader)?;

    let chat_name = root.get("name").and_then(|v| v.as_str()).unwrap_or("?");
    let chat_id = root.get("id").and_then(|v| v.as_i64()).unwrap_or(0);

    let raw_arr = root
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or("no 'messages' array in export")?;

    println!("Chat: {chat_name} (id={chat_id})");
    println!("Total raw messages: {}", raw_arr.len());

    let known: HashSet<&'static str> = KNOWN_KEYS.iter().copied().collect();

    let mut messages: Vec<Message> = Vec::with_capacity(raw_arr.len());
    let mut unknown_keys: HashSet<String> = HashSet::new();
    let mut kinds: HashMap<&'static str, usize> = HashMap::new();
    let mut failures = 0usize;

    for v in raw_arr {
        if let Some(obj) = v.as_object() {
            for k in obj.keys() {
                if !known.contains(k.as_str()) {
                    unknown_keys.insert(k.clone());
                }
            }
        }

        let raw: RawMessage = match serde_json::from_value(v.clone()) {
            Ok(r) => r,
            Err(e) => {
                failures += 1;
                eprintln!("Failed to parse message id={:?}: {e}", v.get("id"));
                continue;
            }
        };

        let msg = Message::from_raw(raw);
        *kinds
            .entry(match &msg {
                Message::Text(_) => "text",
                Message::Photo(_) => "photo",
                Message::Video(_) => "video",
                Message::Sticker(_) => "sticker",
                Message::File(_) => "file",
                Message::Service(_) => "service",
                Message::Unknown(_) => "unknown",
            })
            .or_default() += 1;
        messages.push(msg);
    }

    println!("\nParsed: {}", messages.len());
    if failures > 0 {
        println!("Failures: {failures}");
    }

    println!("\nBy kind:");
    let mut kinds_vec: Vec<_> = kinds.into_iter().collect();
    kinds_vec.sort_by(|a, b| b.1.cmp(&a.1));
    for (k, n) in kinds_vec {
        println!("  {k:>8}: {n}");
    }

    if !unknown_keys.is_empty() {
        let mut uk: Vec<_> = unknown_keys.into_iter().collect();
        uk.sort();
        println!("\nUnknown keys:");
        for k in uk {
            println!("  {k}");
        }
    }

    let compact = compact_dump(&messages);
    std::fs::write(&out_path, &compact)?;

    println!(
        "\nWrote {out_path}: {} bytes, {} lines",
        compact.len(),
        compact.lines().count(),
    );

    println!("\n--- head ---");
    for line in compact.lines().take(35) {
        // табы → визуальные стрелки для читаемости в терминале
        println!("{}", line.replace('\t', "→"));
    }

    Ok(())
}
