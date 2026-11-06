#[cfg(target_os = "emscripten")]
use std::cell::RefCell;
#[cfg(target_os = "emscripten")]
use std::collections::HashMap;

use web_shell::{FIXTURE_ASSET_ROOT, FIXTURE_MANIFEST_PATH, FIXTURE_MODULES, fixture_module_path};

fn paths() -> Vec<String> {
    std::iter::once(FIXTURE_MANIFEST_PATH.to_owned())
        .chain(FIXTURE_MODULES.iter().map(|name| fixture_module_path(name)))
        .collect()
}

fn module_key(url: &str) -> Result<&str, String> {
    url.strip_prefix(FIXTURE_ASSET_ROOT)
        .ok_or_else(|| format!("fixture asset outside {FIXTURE_ASSET_ROOT}: {url}"))
}

#[cfg(target_os = "emscripten")]
type Loaded = Result<(String, HashMap<String, String>), String>;

#[cfg(target_os = "emscripten")]
thread_local! {
    static READY: RefCell<Option<Loaded>> = const { RefCell::new(None) };
}

#[cfg(target_os = "emscripten")]
pub(crate) fn start() {
    load_at(0, String::new(), HashMap::new());
}

#[cfg(target_os = "emscripten")]
pub(crate) fn take_ready() -> Option<Loaded> {
    READY.with(|cell| cell.borrow_mut().take())
}

#[cfg(target_os = "emscripten")]
fn load_at(index: usize, manifest: String, mut modules: HashMap<String, String>) {
    let urls = paths();
    if index == urls.len() {
        READY.with(|cell| *cell.borrow_mut() = Some(Ok((manifest, modules))));
        return;
    }
    let url = urls[index].clone();
    crate::browser_fetch::fetch(&urls[index], move |result| {
        let text = result.and_then(|bytes| {
            String::from_utf8(bytes)
                .map_err(|error| format!("{url}: invalid UTF-8: {error}"))
        });
        match text {
            Ok(text) if index == 0 => load_at(index + 1, text, modules),
            Ok(text) => match module_key(&url) {
                Ok(key) => {
                    modules.insert(key.to_owned(), text);
                    load_at(index + 1, manifest, modules);
                }
                Err(error) => READY.with(|cell| *cell.borrow_mut() = Some(Err(error))),
            },
            Err(error) => READY.with(|cell| *cell.borrow_mut() = Some(Err(error))),
        }
    });
}

#[cfg(test)]
mod tests {
    use web_shell::{FIXTURE_MANIFEST_PATH, FIXTURE_MODULES, fixture_module_path};

    #[test]
    fn fixture_loader_when_started_lists_manifest_then_all_authored_modules() {
        let urls = super::paths();
        assert_eq!(urls[0], FIXTURE_MANIFEST_PATH);
        assert_eq!(urls.len(), FIXTURE_MODULES.len() + 1);
        for (name, url) in FIXTURE_MODULES.iter().zip(&urls[1..]) {
            assert_eq!(url, &fixture_module_path(name));
            assert_eq!(
                super::module_key(url).unwrap(),
                format!("scripts/{}.lua", name.replace('.', "/"))
            );
        }
    }
}
