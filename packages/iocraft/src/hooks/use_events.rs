use crate::{Hook, Hooks};
use core::{
    any::{Any, TypeId},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use futures::task::AtomicWaker;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, OnceLock, Weak},
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// An event being delivered to a component's subscribers.
///
/// Subscribers on the dispatching component run first, in hook order, followed by
/// subscribers on each parent component. Changes to `data` are visible to later subscribers.
#[derive(Debug)]
pub struct Event<T> {
    /// The event's payload.
    pub data: T,
    propagation_stopped: bool,
}

impl<T> Event<T> {
    /// Prevents any remaining subscribers, including those on this component, from
    /// receiving this event.
    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }
}

/// Dispatches events from the component where it was created.
///
/// Events are queued for the render loop, not delivered synchronously. A dispatcher
/// may be cloned and used by other threads. Dispatching after its component has
/// unmounted discards the event. Payloads do not need to implement `Clone` or `Sync`.
pub struct EventDispatcher<T: Send + 'static> {
    target: Weak<EventTarget>,
    _marker: PhantomData<fn(T)>,
}

impl<T: Send + 'static> Clone for EventDispatcher<T> {
    fn clone(&self) -> Self {
        Self {
            target: self.target.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T: Send + 'static> EventDispatcher<T> {
    /// Queues an event for this component and its ancestors. Unrelated components
    /// and subscribers for other payload types do not receive it.
    pub fn dispatch(&self, data: T) {
        if let Some(target) = self.target.upgrade() {
            target.deliver(
                TypeId::of::<T>(),
                0,
                Box::new(Event {
                    data,
                    propagation_stopped: false,
                }),
            );
        }
    }
}

impl<T: Send + 'static> Hook for EventDispatcher<T> {}

/// Hooks for typed events that bubble from a component to its ancestors.
///
/// ```
/// # use iocraft::prelude::*;
/// struct Increment(u32);
///
/// #[component]
/// fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
///     let mut count = hooks.use_state(|| 0_u32);
///     hooks.use_events::<Increment>(move |event| {
///         count += event.data.0;
///         event.stop_propagation();
///     });
///     let dispatcher = hooks.use_event_dispatcher::<Increment>();
///
///     element! {
///         Button(handler: move |_| dispatcher.dispatch(Increment(1)), has_focus: true) {
///             Text(content: format!("Count: {}", count))
///         }
///     }
/// }
/// ```
pub trait UseEvents: private::Sealed {
    /// Subscribes to events with this payload type on the component and its descendants.
    ///
    /// The callback is refreshed each render. Multiple subscriptions on one component
    /// run in hook order; [`Event::stop_propagation`] stops both later subscriptions and
    /// ancestor delivery. Events are delivered while polling the render loop, so this
    /// hook does not run callbacks for static renders such as `print()`.
    fn use_events<T: Send + 'static>(&mut self, f: impl FnMut(&mut Event<T>) + Send + 'static);

    /// Returns a dispatcher whose origin remains this component, even if the handle
    /// is passed to a child or used by another thread.
    fn use_event_dispatcher<T: Send + 'static>(&mut self) -> EventDispatcher<T>;
}

impl UseEvents for Hooks<'_, '_> {
    fn use_events<T: Send + 'static>(&mut self, f: impl FnMut(&mut Event<T>) + Send + 'static) {
        let path = self.event_path;
        let hook = self.use_hook(|| {
            let target = path.target();
            let inbox = Arc::new(EventInbox::default());
            target
                .subscribers
                .lock()
                .unwrap()
                .entry(TypeId::of::<T>())
                .or_default()
                .push(Arc::downgrade(&inbox));
            UseEventsImpl {
                inbox,
                callback: None,
            }
        });
        hook.callback = Some(Box::new(f));
    }

    fn use_event_dispatcher<T: Send + 'static>(&mut self) -> EventDispatcher<T> {
        let path = self.event_path;
        self.use_hook(|| EventDispatcher {
            target: Arc::downgrade(path.target()),
            _marker: PhantomData,
        })
        .clone()
    }
}

// Keep the component path on the update stack. Event routing allocates only when
// a component actually requests a subscriber or dispatcher.
#[derive(Default)]
pub(crate) struct EventScope(OnceLock<Arc<EventTarget>>);

pub(crate) struct EventPath<'a> {
    scope: &'a EventScope,
    parent: Option<&'a EventPath<'a>>,
}

impl<'a> EventPath<'a> {
    pub(crate) fn new(scope: &'a EventScope, parent: Option<&'a EventPath<'a>>) -> Self {
        Self { scope, parent }
    }

    fn target(&self) -> &Arc<EventTarget> {
        self.scope.0.get_or_init(|| {
            Arc::new(EventTarget {
                parent: self
                    .parent
                    .map(|parent| Arc::downgrade(parent.target()))
                    .unwrap_or_default(),
                subscribers: Mutex::new(HashMap::new()),
            })
        })
    }
}

struct EventTarget {
    parent: Weak<EventTarget>,
    subscribers: Mutex<HashMap<TypeId, Vec<Weak<EventInbox>>>>,
}

impl EventTarget {
    fn deliver(self: Arc<Self>, kind: TypeId, mut next: usize, payload: Box<dyn Any + Send>) {
        let mut target = self;
        loop {
            let subscriber = {
                let subscribers = target.subscribers.lock().unwrap();
                subscribers.get(&kind).and_then(|subscribers| {
                    subscribers
                        .iter()
                        .enumerate()
                        .skip(next)
                        .find_map(|(index, inbox)| inbox.upgrade().map(|inbox| (index, inbox)))
                })
            };
            if let Some((index, inbox)) = subscriber {
                inbox.push(PendingEvent {
                    payload,
                    kind,
                    target: Arc::downgrade(&target),
                    next_subscriber: index + 1,
                });
                return;
            }
            match target.parent.upgrade() {
                Some(parent) => target = parent,
                None => return,
            }
            next = 0;
        }
    }
}

struct PendingEvent {
    payload: Box<dyn Any + Send>,
    kind: TypeId,
    target: Weak<EventTarget>,
    next_subscriber: usize,
}

#[derive(Default)]
struct EventInbox {
    events: Mutex<VecDeque<PendingEvent>>,
    waker: AtomicWaker,
}

impl EventInbox {
    fn push(&self, event: PendingEvent) {
        self.events.lock().unwrap().push_back(event);
        self.waker.wake();
    }

    fn pop(&self) -> Option<PendingEvent> {
        self.events.lock().unwrap().pop_front()
    }
}

type EventCallback<T> = Box<dyn FnMut(&mut Event<T>) + Send>;

struct UseEventsImpl<T> {
    inbox: Arc<EventInbox>,
    callback: Option<EventCallback<T>>,
}

impl<T: Send + 'static> Hook for UseEventsImpl<T> {
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        this.inbox.waker.register(cx.waker());
        while let Some(pending) = this.inbox.pop() {
            let mut event = pending
                .payload
                .downcast::<Event<T>>()
                .expect("event payload must match the subscriber type");
            this.callback
                .as_mut()
                .expect("event callback is installed during component update")(
                &mut event
            );
            if !event.propagation_stopped {
                if let Some(target) = pending.target.upgrade() {
                    target.deliver(pending.kind, pending.next_subscriber, event);
                }
            }
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use futures::stream::{self, StreamExt};
    use macro_rules_attribute::apply;
    use smol_macros::test;
    use std::{
        cell::Cell,
        sync::{Arc, Mutex},
    };

    struct Change {
        amount: Cell<i32>,
        consume: bool,
    }

    impl Change {
        fn new(amount: i32, consume: bool) -> Self {
            Self {
                amount: Cell::new(amount),
                consume,
            }
        }
    }

    #[derive(Default)]
    struct Controls {
        origin: Mutex<Option<EventDispatcher<Change>>>,
        root: Mutex<Option<EventDispatcher<Change>>>,
        shown: Mutex<Option<State<bool>>>,
    }

    #[component]
    fn Origin(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let controls = hooks.use_context::<Arc<Controls>>().clone();
        let mut handled = hooks.use_state(|| 0);
        let mut total = hooks.use_state(|| 0);
        let dispatcher = hooks.use_event_dispatcher::<Change>();
        *controls.origin.lock().unwrap() = Some(dispatcher.clone());
        hooks.use_events::<Change>(move |event| {
            handled += 1;
            if event.data.amount.get() == 1 {
                dispatcher.dispatch(Change::new(2, false));
            }
            event.data.amount.set(event.data.amount.get() * 2);
            if event.data.consume {
                event.stop_propagation();
            }
        });
        hooks.use_events::<Change>(move |event| {
            total += event.data.amount.get();
            event.data.amount.set(event.data.amount.get() + 3);
        });
        element! {
            View(flex_direction: FlexDirection::Column) {
                Text(content: handled.to_string())
                Text(content: total.to_string())
            }
        }
    }

    #[component]
    fn Parent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut total = hooks.use_state(|| 0);
        hooks.use_events::<Change>(move |event| {
            total += event.data.amount.get();
        });
        element! {
            View(flex_direction: FlexDirection::Column) {
                Text(content: total.to_string())
                Origin
            }
        }
    }

    #[component]
    fn Sibling(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut total = hooks.use_state(|| 0);
        hooks.use_events::<Change>(move |event| {
            total += event.data.amount.get();
        });
        element!(Text(content: total.to_string()))
    }

    #[component]
    fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let controls = hooks.use_context::<Arc<Controls>>().clone();
        let shown = hooks.use_state(|| true);
        let mut total = hooks.use_state(|| 0);
        let mut other = hooks.use_state(|| 0);
        *controls.shown.lock().unwrap() = Some(shown);
        *controls.root.lock().unwrap() = Some(hooks.use_event_dispatcher::<Change>());
        hooks.use_events::<Change>(move |event| {
            total += event.data.amount.get();
        });
        hooks.use_events::<u32>(move |event| {
            other += event.data as i32;
        });
        element! {
            View(flex_direction: FlexDirection::Column) {
                Text(content: total.to_string())
                #(if shown.get() {
                    Some(element!(Parent))
                } else {
                    None
                })
                Sibling
                Text(content: other.to_string())
            }
        }
    }

    fn values(canvas: Canvas) -> Vec<i32> {
        canvas
            .to_string()
            .lines()
            .map(|line| line.trim().parse().unwrap())
            .collect()
    }

    #[apply(test!)]
    async fn test_bubbling_order_consumption_and_reentrant_dispatch() {
        let controls = Arc::new(Controls::default());
        let mut app = element! {
            ContextProvider(value: Context::owned(controls.clone())) {
                App
            }
        };
        let mut frames =
            app.mock_terminal_render_loop(MockTerminalConfig::with_events(stream::pending()));
        assert_eq!(values(frames.next().await.unwrap()), [0, 0, 0, 0, 0, 0]);

        let dispatcher = controls.origin.lock().unwrap().clone().unwrap();
        dispatcher.dispatch(Change::new(1, false));
        assert_eq!(values(frames.next().await.unwrap()), [12, 12, 2, 6, 0, 0]);

        // Change is neither Clone nor Sync, but it can be dispatched from a worker.
        let worker_dispatcher = dispatcher.clone();
        std::thread::spawn(move || worker_dispatcher.dispatch(Change::new(10, false)))
            .join()
            .unwrap();
        assert_eq!(values(frames.next().await.unwrap()), [35, 35, 3, 26, 0, 0]);

        dispatcher.dispatch(Change::new(100, true));
        assert_eq!(values(frames.next().await.unwrap()), [35, 35, 4, 26, 0, 0]);
    }

    #[apply(test!)]
    async fn test_unmounted_dispatcher_does_not_reach_former_ancestors() {
        let controls = Arc::new(Controls::default());
        let mut app = element! {
            ContextProvider(value: Context::owned(controls.clone())) {
                App
            }
        };
        let mut frames =
            app.mock_terminal_render_loop(MockTerminalConfig::with_events(stream::pending()));
        assert_eq!(values(frames.next().await.unwrap()), [0, 0, 0, 0, 0, 0]);
        let old_dispatcher = controls.origin.lock().unwrap().clone().unwrap();

        controls.shown.lock().unwrap().as_mut().unwrap().set(false);
        assert_eq!(values(frames.next().await.unwrap()), [0, 0, 0]);

        old_dispatcher.dispatch(Change::new(100, false));
        controls
            .root
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .dispatch(Change::new(7, false));
        assert_eq!(values(frames.next().await.unwrap()), [7, 0, 0]);
    }
}
