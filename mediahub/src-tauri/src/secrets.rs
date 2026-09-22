const SERVICE: &str = "mediahub";

fn entry(account: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, account).map_err(|_| "Sicherer Speicher nicht verfügbar.".to_string())
}

pub fn set_secret(account: &str, value: &str) -> Result<(), String> {
    entry(account)?
        .set_password(value)
        .map_err(|_| "Sicherer Speicher nicht verfügbar.".to_string())
}

pub fn get_secret(account: &str) -> Result<Option<String>, String> {
    match entry(account)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("Sicherer Speicher nicht verfügbar.".into()),
    }
}

pub fn delete_secret(account: &str) -> Result<(), String> {
    match entry(account)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("Sicherer Speicher nicht verfügbar.".into()),
    }
}

pub fn has_secret(account: &str) -> bool {
    matches!(get_secret(account), Ok(Some(_)))
}
