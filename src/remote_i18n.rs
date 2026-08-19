use rust_i18n::Backend;
use std::{
    fs, 
    borrow::Cow, 
    collections::HashMap
};

pub struct RemoteI18n {
    trs: HashMap<String, HashMap<String, String>>,
}

impl RemoteI18n {
    pub fn new() -> Self {
        let data_dir = dirs::data_dir().expect("No data_dir directory found").join(super::APP_FOLDER);
        let path = data_dir.join("locales").join("app.yml");
        if !path.exists() {
            return Self { trs: std::collections::HashMap::new() };
        }

        let trs = fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!(e))
            .and_then(|content| {
                serde_yaml_ng::from_str::<HashMap<String, HashMap<String, String>>>(&content)
                    .map_err(|e| anyhow::anyhow!(e))
            });

        // Check for errors
        let final_trs = match trs {
            Ok(parsed_maps) => parsed_maps,
            Err(err) => {
                tracing::warn!("Failed to load custom locales: {:?}", err);
                HashMap::new()
            }
        };

        return Self {
            trs: final_trs
        };
    }
}

impl Backend for RemoteI18n {
    fn available_locales(&self) -> Vec<Cow<'_, str>> {
        return self.trs.keys().map(|k| Cow::from(k.as_str())).collect();
    }

    fn translate(&self, locale: &str, key: &str) -> Option<Cow<'_, str>> {
        return self.trs.get(locale)?.get(key).map(|k| Cow::from(k.as_str()));
    }

    fn messages_for_locale(&self, locale: &str) -> Option<Vec<(Cow<'static, str>, Cow<'static, str>)>> {
        // translations for locale
        // works only when the key is a language name, but this format is not supported by the main crate.
        let locale_map = self.trs.get(locale)?;

        // key-value to Vec<(Cow, Cow)>
        let messages: Vec<(Cow<'static, str>, Cow<'static, str>)> = locale_map
            .iter()
            .map(|(k, v)| {
                (
                    Cow::Owned(k.clone()),
                    Cow::Owned(v.clone()),
                )
            })
            .collect();

        Some(messages)
    }
}
