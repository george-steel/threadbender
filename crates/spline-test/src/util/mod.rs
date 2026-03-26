use std::mem::replace;

use leptos::prelude::{guards::{ReadGuard}, *};

pub mod resize;


struct MailboxInner<T> where
    T: Send + Sync + 'static
{
    signal: ArcSignal<T>,
    dirty: bool,
}

pub struct Mailbox<T> where
    T: Send + Sync + 'static,
{
    inner: ArenaItem<MailboxInner<T>>,
}

// Copy regardless of tinnte type, unlike the Derive
impl<T> Copy for Mailbox<T> where T: Send + Sync + 'static {}

impl<T> Clone for Mailbox<T> where
    T: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Mailbox<T> where
    T: Send + Sync + 'static,
{
    pub fn new_scoped(sig: ArcSignal<T>, trigger: ArcTrigger) -> Self {
        let mailbox = ArenaItem::new(MailboxInner {
            signal: sig.clone(),
            dirty: true,
        });

        Effect::new(move|| {
            sig.track();
            let new_write = mailbox.try_update_value(|inner| {
                !replace(&mut inner.dirty, true)
            });
            if new_write == Some(true) {
                trigger.notify();
            }
        });
        Mailbox { inner: mailbox }
    }

    pub fn read_new(&self) -> Option<ReadGuard<T, SignalReadGuard<T, SyncStorage>>> {
        let maybe_sig = self.inner.try_update_value(|inner|{
            if inner.dirty {
                inner.dirty = false;
                Some(inner.signal.clone())
            } else {
                None
            }
        }).flatten();

        if let Some(sig) = maybe_sig {
            Some(sig.read_untracked())
        } else {
            None
        }
    }
}

impl<T> Mailbox<T> where
    T: Send + Sync + Clone + 'static,
{
    pub fn get_new(&self) -> Option<T> {
        let maybe_sig = self.inner.try_update_value(|inner|{
            if inner.dirty {
                inner.dirty = false;
                Some(inner.signal.clone())
            } else {
                None
            }
        }).flatten();

        if let Some(sig) = maybe_sig {
            Some(sig.get_untracked())
        } else {
            None
        }
    }
}