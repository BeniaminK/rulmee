use uzers::all_users;
use uzers::os::unix::UserExt;

#[derive(Debug, Clone)]
pub struct LocalUser {
    pub username: String,
    pub display_name: String,
    pub shell: String,
}

pub fn get_human_users() -> Vec<LocalUser> {
    let mut users = Vec::new();
    let iter = unsafe { all_users() };

    for user in iter {
        let home_dir = user.home_dir();
        if home_dir.starts_with("/home/") {
            let username = user.name().to_string_lossy().to_string();
            let gecos_str = user.gecos().to_string_lossy();
            let first_part = gecos_str.split(',').next().unwrap_or("").trim();
            let display_name = if first_part.is_empty() {
                username.clone()
            } else {
                first_part.to_string()
            };
            users.push(LocalUser {
                username,
                display_name,
                shell: user.shell().to_string_lossy().to_string(),
            });
        }
    }

    users
}
