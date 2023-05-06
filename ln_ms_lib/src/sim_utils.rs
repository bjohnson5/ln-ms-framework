// External Modules
use chrono::Local;
use std::fs;

pub fn get_current_time() -> String {
    let date = Local::now();
    format!("{}", date.format("[%Y-%m-%d][%H:%M:%S]"))
}

pub fn cleanup_sensei(sensei_data_dir_main: String) {
    fs::remove_file(sensei_data_dir_main.clone() + "/sensei.db").expect("File delete failed");
    fs::remove_file(sensei_data_dir_main.clone() + "/sensei.db-shm").expect("File delete failed");
    fs::remove_file(sensei_data_dir_main.clone() + "/sensei.db-wal").expect("File delete failed");
    fs::remove_dir_all(sensei_data_dir_main.clone() + "/logs").expect("File delete failed");
    fs::remove_dir_all(sensei_data_dir_main.clone() + "/regtest").expect("File delete failed");
}