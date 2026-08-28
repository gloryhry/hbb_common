use std::{fs, path::Path};

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    if source.contains(to) {
        return;
    }

    let count = source.matches(from).count();
    assert_eq!(
        count, 1,
        "Unable to apply locked RustDesk server policy patch '{label}': expected exactly one source match, found {count}. Upstream config.rs likely changed; update build.rs before releasing."
    );
    *source = source.replacen(from, to, 1);
}

fn apply_locked_server_policy() {
    let path = Path::new("src/config.rs");
    let mut source = fs::read_to_string(path).expect("Failed to read src/config.rs");
    let original = source.clone();

    replace_once(
        &mut source,
        r#"    pub static ref BUILTIN_SETTINGS: RwLock<HashMap<String, String>> = Default::default();"#,
        r#"    pub static ref BUILTIN_SETTINGS: RwLock<HashMap<String, String>> = RwLock::new(HashMap::from([
        (keys::OPTION_HIDE_SERVER_SETTINGS.to_owned(), "Y".to_owned()),
        (
            keys::OPTION_ALLOW_DEEP_LINK_SERVER_SETTINGS.to_owned(),
            "N".to_owned(),
        ),
    ]));"#,
        "hide server settings",
    );

    replace_once(
        &mut source,
        r#"    pub fn get_rendezvous_server() -> String {
        let mut rendezvous_server = EXE_RENDEZVOUS_SERVER.read().unwrap().clone();
        if rendezvous_server.is_empty() {
            rendezvous_server = Self::get_option("custom-rendezvous-server");
        }
        if rendezvous_server.is_empty() {
            rendezvous_server = PROD_RENDEZVOUS_SERVER.read().unwrap().clone();
        }
        if rendezvous_server.is_empty() {
            rendezvous_server = CONFIG2.read().unwrap().rendezvous_server.clone();
        }
        if rendezvous_server.is_empty() {
            rendezvous_server = Self::get_rendezvous_servers()
                .drain(..)
                .next()
                .unwrap_or_default();
        }
        if !rendezvous_server.contains(':') {
            rendezvous_server = format!("{rendezvous_server}:{RENDEZVOUS_PORT}");
        }
        rendezvous_server
    }

    pub fn get_rendezvous_servers() -> Vec<String> {
        let s = EXE_RENDEZVOUS_SERVER.read().unwrap().clone();
        if !s.is_empty() {
            return vec![s];
        }
        let s = Self::get_option("custom-rendezvous-server");
        if !s.is_empty() {
            return vec![s];
        }
        let s = PROD_RENDEZVOUS_SERVER.read().unwrap().clone();
        if !s.is_empty() {
            return vec![s];
        }
        let serial_obsolute = CONFIG2.read().unwrap().serial > SERIAL;
        if serial_obsolute {
            let ss: Vec<String> = Self::get_option("rendezvous-servers")
                .split(',')
                .filter(|x| x.contains('.'))
                .map(|x| x.to_owned())
                .collect();
            if !ss.is_empty() {
                return ss;
            }
        }
        return RENDEZVOUS_SERVERS.iter().map(|x| x.to_string()).collect();
    }"#,
        r#"    pub fn get_rendezvous_server() -> String {
        let mut rendezvous_server = RENDEZVOUS_SERVERS
            .first()
            .copied()
            .unwrap_or_default()
            .to_owned();
        if !rendezvous_server.contains(':') {
            rendezvous_server = format!("{rendezvous_server}:{RENDEZVOUS_PORT}");
        }
        rendezvous_server
    }

    pub fn get_rendezvous_servers() -> Vec<String> {
        RENDEZVOUS_SERVERS.iter().map(|x| x.to_string()).collect()
    }"#,
        "force rendezvous server",
    );

    replace_once(
        &mut source,
        r#"    pub fn get_options() -> HashMap<String, String> {
        let mut res = DEFAULT_SETTINGS.read().unwrap().clone();
        res.extend(CONFIG2.read().unwrap().options.clone());
        res.extend(OVERWRITE_SETTINGS.read().unwrap().clone());
        res
    }

    #[inline]
    fn purify_options(v: &mut HashMap<String, String>) {
        v.retain(|k, v| is_option_can_save(&OVERWRITE_SETTINGS, k, &DEFAULT_SETTINGS, v));
    }

    pub fn set_options(mut v: HashMap<String, String>) {
        Self::purify_options(&mut v);
        let mut config = CONFIG2.write().unwrap();
        if config.options == v {
            return;
        }
        config.options = v;
        config.store();
    }

    pub fn get_option(k: &str) -> String {
        get_or(
            &OVERWRITE_SETTINGS,
            &CONFIG2.read().unwrap().options,
            &DEFAULT_SETTINGS,
            k,
        )
        .unwrap_or_default()
    }

    pub fn get_bool_option(k: &str) -> bool {
        option2bool(k, &Self::get_option(k))
    }

    pub fn set_option(k: String, v: String) {
        if !is_option_can_save(&OVERWRITE_SETTINGS, &k, &DEFAULT_SETTINGS, &v) {
            let mut config = CONFIG2.write().unwrap();
            if config.options.remove(&k).is_some() {
                config.store();
            }
            return;
        }
        let mut config = CONFIG2.write().unwrap();
        let v2 = if v.is_empty() { None } else { Some(&v) };
        if v2 != config.options.get(&k) {
            if v2.is_none() {
                config.options.remove(&k);
            } else {
                config.options.insert(k, v);
            }
            config.store();
        }
    }"#,
        r#"    fn get_locked_server_option(k: &str) -> Option<String> {
        let server = || {
            RENDEZVOUS_SERVERS
                .first()
                .copied()
                .unwrap_or_default()
                .to_owned()
        };

        if k == keys::OPTION_CUSTOM_RENDEZVOUS_SERVER || k == "rendezvous-servers" {
            Some(server())
        } else if k == keys::OPTION_RELAY_SERVER {
            Some(server())
        } else if k == keys::OPTION_API_SERVER {
            Some(String::new())
        } else if k == keys::OPTION_KEY {
            Some(RS_PUB_KEY.to_owned())
        } else {
            None
        }
    }

    pub fn get_options() -> HashMap<String, String> {
        let mut res = DEFAULT_SETTINGS.read().unwrap().clone();
        res.extend(CONFIG2.read().unwrap().options.clone());
        res.extend(OVERWRITE_SETTINGS.read().unwrap().clone());
        for key in [
            keys::OPTION_CUSTOM_RENDEZVOUS_SERVER,
            "rendezvous-servers",
            keys::OPTION_RELAY_SERVER,
            keys::OPTION_API_SERVER,
            keys::OPTION_KEY,
        ] {
            if let Some(value) = Self::get_locked_server_option(key) {
                res.insert(key.to_owned(), value);
            }
        }
        res
    }

    #[inline]
    fn purify_options(v: &mut HashMap<String, String>) {
        v.retain(|k, v| {
            Self::get_locked_server_option(k).is_none()
                && is_option_can_save(&OVERWRITE_SETTINGS, k, &DEFAULT_SETTINGS, v)
        });
    }

    pub fn set_options(mut v: HashMap<String, String>) {
        Self::purify_options(&mut v);
        let mut config = CONFIG2.write().unwrap();
        if config.options == v {
            return;
        }
        config.options = v;
        config.store();
    }

    pub fn get_option(k: &str) -> String {
        if let Some(value) = Self::get_locked_server_option(k) {
            return value;
        }
        get_or(
            &OVERWRITE_SETTINGS,
            &CONFIG2.read().unwrap().options,
            &DEFAULT_SETTINGS,
            k,
        )
        .unwrap_or_default()
    }

    pub fn get_bool_option(k: &str) -> bool {
        option2bool(k, &Self::get_option(k))
    }

    pub fn set_option(k: String, v: String) {
        if Self::get_locked_server_option(&k).is_some() {
            let mut config = CONFIG2.write().unwrap();
            if config.options.remove(&k).is_some() {
                config.store();
            }
            return;
        }
        if !is_option_can_save(&OVERWRITE_SETTINGS, &k, &DEFAULT_SETTINGS, &v) {
            let mut config = CONFIG2.write().unwrap();
            if config.options.remove(&k).is_some() {
                config.store();
            }
            return;
        }
        let mut config = CONFIG2.write().unwrap();
        let v2 = if v.is_empty() { None } else { Some(&v) };
        if v2 != config.options.get(&k) {
            if v2.is_none() {
                config.options.remove(&k);
            } else {
                config.options.insert(k, v);
            }
            config.store();
        }
    }"#,
        "lock server and key options",
    );

    if source != original {
        fs::write(path, source).expect("Failed to apply locked RustDesk server policy");
    }
}

fn main() {
    // Keep these directives explicit so the build script does not rerun merely because it
    // intentionally patched src/config.rs during this build.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=protos/rendezvous.proto");
    println!("cargo:rerun-if-changed=protos/message.proto");

    apply_locked_server_policy();

    let out_dir = format!("{}/protos", std::env::var("OUT_DIR").unwrap());

    std::fs::create_dir_all(&out_dir).unwrap();

    protobuf_codegen::Codegen::new()
        .pure()
        .out_dir(out_dir)
        .inputs(["protos/rendezvous.proto", "protos/message.proto"])
        .include("protos")
        .customize(protobuf_codegen::Customize::default().tokio_bytes(true))
        .run()
        .expect("Codegen failed.");
}
