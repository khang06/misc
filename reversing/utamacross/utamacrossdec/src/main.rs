use log::{info, warn};

fn generate_key(seed: u32) -> Vec<u8> {
    let mut out = vec![0; 1024];

    let mut state = seed;
    for x in out.iter_mut().take(1024) {
        let temp = state ^ (state << 13) ^ ((state ^ (state << 13)) >> 17);
        state = temp ^ (temp * 32);
        *x = (state >> 3 & 0xFF) as u8;
    }

    out
}

fn xedec_dec(data: &mut [u8], key: &[u8]) {
    assert_eq!(key.len(), 1024);

    let mut key_pos = data.len() as i32;
    for x in data {
        key_pos = 7 * key_pos + 1;
        *x ^= key[(key_pos & 0x3FF) as usize];
    }
}

#[derive(Default)]
struct DecryptionRules {
    rules: Vec<(regex::Regex, Vec<u8>)>,
}

impl DecryptionRules {
    pub fn add(&mut self, regex: &str, seed: u32) {
        self.rules
            .push((regex::Regex::new(regex).unwrap(), generate_key(seed)));
    }

    pub fn get_key(&self, path: &str) -> Option<&[u8]> {
        for x in &self.rules {
            if x.0.is_match(path) {
                return Some(&x.1);
            }
        }

        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let mut rules = DecryptionRules::default();
    rules.add(r".usm$", 0x0);
    rules.add(r"/cd/.+", 0x38990d6b);
    rules.add(r"/ct/ba/.+", 0x339fd546);
    rules.add(r"/ct/bg/.+", 0x187088ed);
    rules.add(r"/ct/im/.+", 0x228e1dbf);
    rules.add(r"/ct/lo/.+", 0x16aba5d1);
    rules.add(r"/ct/mc/.+", 0x32dede0c);
    rules.add(r"/ct/rk/.+", 0xbf78826);
    rules.add(r"/ct/sc/.+", 0x11c78866);
    rules.add(r"/ct/sk/.+", 0x21cc3b9d);
    rules.add(r"/ct/.+", 0x339fd546);
    rules.add(r"/dv/.+", 0x2d5ff754);
    rules.add(r"/ev/.+", 0x20461933);
    rules.add(r"/gc/.+", 0x34512f4a);
    rules.add(r"/gm/.+", 0x36427614);
    rules.add(r"/handmode/.+", 0xd37549f);
    rules.add(r"/ly/.+", 0x22cc70e8);
    rules.add(r"/mc/.+", 0xad6145d);
    rules.add(r"/mn/.+", 0x16703d1f);
    rules.add(r"/msg/.+", 0x16b60b1b);
    rules.add(r"/st/.+", 0x28876611);
    rules.add(r"/vl/.+", 0x3205d7d7);
    rules.add(r".xab$", 0x15ab17a1);

    let args: Vec<String> = std::env::args().collect();
    let input_path = std::path::Path::new(&args[1]);
    for entry in walkdir::WalkDir::new(input_path)
        .into_iter()
        .filter_map(|x| x.ok())
        .filter(|x| x.metadata().ok().filter(|x| x.is_file()).is_some())
    {
        // pretty inefficient but there aren't enough files for me to care
        let stripped = "/".to_owned()
            + &entry
                .path()
                .strip_prefix(input_path)?
                .as_os_str()
                .to_string_lossy()
                .replace('\\', "/");
        let key = rules.get_key(&stripped);
        if let Some(key) = key {
            info!("Decrypting {}", stripped);

            let mut file = std::fs::read(entry.path())?;
            xedec_dec(&mut file, &key);

            let output_path = "./decrypt_output".to_owned() + &stripped;
            let output_path = std::path::Path::new(&output_path);
            std::fs::create_dir_all(output_path.parent().unwrap())?;
            std::fs::write(&output_path, &file)?;
        } else {
            warn!("{} has no key!", stripped);
        }
    }

    Ok(())
}
