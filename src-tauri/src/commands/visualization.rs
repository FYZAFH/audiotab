use std::fs;

#[tauri::command]
pub async fn get_ringbuffer_data() -> Result<Vec<u8>, String> {
    let path = "/tmp/audiotab_ringbuf";

    fs::read(path).map_err(|e| format!("Failed to read ring buffer: {}", e))
}
