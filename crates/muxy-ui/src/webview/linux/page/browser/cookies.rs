use super::{Context, Done, Page, Profile, Result, Runtime};
use polybrowser::{Cookie, webkitgtk::SiteData};

impl Page {
    fn with_data<T: 'static>(
        &self,
        done: Done<T>,
        call: impl FnOnce(&polybrowser::webkitgtk::NativeView, Done<T>) -> Result<()>,
    ) {
        let Some(browser) = &self.browser else {
            done(Err(super::error("not a browser page")));
            return;
        };
        browser.context.operation(done, |done| {
            self.with_native(done, call);
            Ok(())
        });
    }
    pub(in crate::webview::linux) fn browser_cookies(
        &self,
        _runtime: &Runtime,
        url: Option<&str>,
        done: Done<Vec<Cookie>>,
    ) {
        self.with_data(done, |native, done| native.cookies(url, done));
    }
    pub(in crate::webview::linux) fn browser_set_cookie(
        &self,
        _runtime: &Runtime,
        cookie: &Cookie,
        done: Done<()>,
    ) {
        self.with_data(done, |native, done| native.set_cookie(cookie.clone(), done));
    }
    pub(in crate::webview::linux) fn browser_delete_cookies(
        &self,
        _runtime: &Runtime,
        name: &str,
        domain: Option<&str>,
        done: Done<usize>,
    ) {
        self.with_data(done, |native, done| {
            native.delete_cookies(name, domain, done)
        });
    }
    pub(in crate::webview::linux) fn browser_clear_cookies(
        &self,
        _runtime: &Runtime,
        done: Done<()>,
    ) {
        self.with_data(done, polybrowser::webkitgtk::NativeView::clear_cookies);
    }
}
fn store(runtime: &Runtime, profile: &Profile) -> Result<(std::rc::Rc<Context>, SiteData)> {
    let context = Context::get(runtime, profile)?;
    let data = unsafe { SiteData::from_raw(context.native) }?;
    Ok((context, data))
}
pub(in crate::webview::linux) fn clear_site_data(
    runtime: &Runtime,
    profile: &Profile,
    done: Done<()>,
) {
    match store(runtime, profile) {
        Ok((context, store)) => context.operation(done, |done| store.clear(done)),
        Err(failure) => done(Err(failure)),
    }
}
pub(in crate::webview::linux) fn import_cookies(
    runtime: &Runtime,
    profile: &Profile,
    cookies: Vec<Cookie>,
    done: Done<usize>,
) {
    match store(runtime, profile) {
        Ok((context, store)) => context.operation(done, |done| store.import(cookies, done)),
        Err(failure) => done(Err(failure)),
    }
}
