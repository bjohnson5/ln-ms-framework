// External Modules
use chrono::Local;

pub fn get_current_time() -> String {
    let date = Local::now();
    format!("{}", date.format("[%Y-%m-%d][%H:%M:%S]"))
}