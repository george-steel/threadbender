use leptos::prelude::{guards::{ReadGuard}, *};

pub mod resize;


struct MailboxInner<T> where
    T: Send + Sync + 'static
{
    signal: ArcSignal<T>,
    dirty: bool,
}

#[derive(Clone, Copy)]
pub struct Mailbox<T> where
    T: Send + Sync + 'static,
{
    inner: ArenaItem<MailboxInner<T>>,
}

impl<T> Mailbox<T> where
    T: Send + Sync + 'static,
{
    pub fn new_scoped(sig: ArcSignal<T>, trigger: Trigger) -> Self {
        let mailbox = ArenaItem::new(MailboxInner {
            signal: sig.clone(),
            dirty: true,
        });

        ImmediateEffect::new_scoped(move|| {
            sig.track();
            let new_write = mailbox.try_update_value(|inner| {
                if inner.dirty {
                    false
                } else {
                    inner.dirty = true;
                    true
                }
            });
            if new_write == Some(true) {
                trigger.notify();
            }
        });
        Mailbox { inner: mailbox }
    }

    pub fn read_new(&self) -> Option<ReadGuard<T, SignalReadGuard<T, SyncStorage>>> {
        self.inner.try_update_value(|inner|{
            if inner.dirty {
                inner.dirty = false;
                Some(inner.signal.read_untracked())
            } else {
                None
            }
        }).flatten()
    }
}

impl<T> Mailbox<T> where
    T: Send + Sync + Clone + 'static,
{
    pub fn get_new(&self) -> Option<T> {
        self.inner.try_update_value(|inner|{
            if inner.dirty {
                inner.dirty = false;
                Some(inner.signal.get_untracked())
            } else {
                None
            }
        }).flatten()
    }
}