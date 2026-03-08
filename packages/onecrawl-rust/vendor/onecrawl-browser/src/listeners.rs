use std::collections::VecDeque;
use std::fmt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use fnv::FnvHashMap;
use futures::channel::mpsc::{SendError, UnboundedReceiver, UnboundedSender};
use futures::{Sink, Stream};

use onecrawl_protocol::cdp::{Event, EventKind, IntoEventKind};
use onecrawl_browser_types::MethodId;

/// All the currently active listeners
#[derive(Debug, Default)]
pub struct EventListeners {
    /// Tracks the listeners for each event identified by the key
    listeners: FnvHashMap<MethodId, Vec<EventListener>>,
}

impl EventListeners {
    /// Register a subscription for a method
    pub fn add_listener(&mut self, req: EventListenerRequest) {
        let EventListenerRequest {
            listener,
            method,
            kind,
        } = req;
        let subs = self.listeners.entry(method).or_default();
        subs.push(EventListener {
            listener,
            kind,
            queued_events: Default::default(),
        });
    }

    /// Queue in a event that should be send to all listeners
    #[inline]
    pub fn start_send<T: Event>(&mut self, event: T) {
        if let Some(subscriptions) = self.listeners.get_mut(&T::method_id()) {
            let event: Arc<dyn Event> = Arc::new(event);
            subscriptions
                .iter_mut()
                .for_each(|sub| sub.start_send(Arc::clone(&event)));
        }
    }

    /// Try to queue in a new custom event if a listener is registered and the
    /// converting the json value to the registered event type succeeds
    #[inline]
    pub fn try_send_custom(
        &mut self,
        method: &str,
        val: serde_json::Value,
    ) -> serde_json::Result<()> {
        if let Some(subscriptions) = self.listeners.get_mut(method) {
            let event = subscriptions
                .iter()
                .find_map(|sub| {
                    if let EventKind::Custom(conv) = &sub.kind {
                        Some(conv)
                    } else {
                        None
                    }
                })
                .map(|json_to_arc_event| json_to_arc_event(val))
                .transpose()?;
            if let Some(event) = event {
                subscriptions
                    .iter_mut()
                    .filter(|sub| sub.kind.is_custom())
                    .for_each(|sub| sub.start_send(Arc::clone(&event)));
            }
        }
        Ok(())
    }

    /// Drains all queued events and does the housekeeping when the receiver
    /// part of a subscription is dropped
    pub fn poll(&mut self, cx: &mut Context<'_>) {
        for subscriptions in self.listeners.values_mut() {
            subscriptions.retain_mut(|sub| {
                match sub.poll(cx) {
                    Poll::Ready(Err(err)) if err.is_disconnected() => false,
                    _ => true,
                }
            });
        }
    }
}

pub struct EventListenerRequest {
    listener: UnboundedSender<Arc<dyn Event>>,
    method: MethodId,
    kind: EventKind,
}

impl EventListenerRequest {
    pub fn new<T: IntoEventKind>(listener: UnboundedSender<Arc<dyn Event>>) -> Self {
        Self {
            listener,
            method: T::method_id(),
            kind: T::event_kind(),
        }
    }
}

impl fmt::Debug for EventListenerRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventListenerRequest")
            .field("method", &self.method)
            .field("kind", &self.kind)
            .finish()
    }
}

/// Represents a single event listener
pub struct EventListener {
    /// the sender half of the event channel
    listener: UnboundedSender<Arc<dyn Event>>,
    /// currently queued events
    queued_events: VecDeque<Arc<dyn Event>>,
    /// For what kind of event this event is for
    kind: EventKind,
}

impl EventListener {
    /// queue in a new event
    pub fn start_send(&mut self, event: Arc<dyn Event>) {
        self.queued_events.push_back(event)
    }

    /// Drains all queued events and begins the process of sending them to the
    /// sink.
    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SendError>> {
        loop {
            match Sink::poll_ready(Pin::new(&mut self.listener), cx) {
                Poll::Ready(Ok(_)) => {}
                Poll::Ready(Err(err)) => {
                    // disconnected
                    return Poll::Ready(Err(err));
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
            if let Some(event) = self.queued_events.pop_front() {
                if let Err(err) = Sink::start_send(Pin::new(&mut self.listener), event) {
                    return Poll::Ready(Err(err));
                }
            } else {
                return Poll::Ready(Ok(()));
            }
        }
    }
}

impl fmt::Debug for EventListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventListener").finish()
    }
}

/// The receiver part of an event subscription
pub struct EventStream<T: IntoEventKind> {
    events: UnboundedReceiver<Arc<dyn Event>>,
    _marker: PhantomData<T>,
}

impl<T: IntoEventKind> fmt::Debug for EventStream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventStream").finish()
    }
}

impl<T: IntoEventKind> EventStream<T> {
    pub fn new(events: UnboundedReceiver<Arc<dyn Event>>) -> Self {
        Self {
            events,
            _marker: PhantomData,
        }
    }
}

impl<T: IntoEventKind + Unpin> Stream for EventStream<T> {
    type Item = Arc<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let pin = self.get_mut();
        match Stream::poll_next(Pin::new(&mut pin.events), cx) {
            Poll::Ready(Some(event)) => {
                if let Ok(e) = event.into_any_arc().downcast() {
                    Poll::Ready(Some(e))
                } else {
                    Poll::Pending
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::{SinkExt, StreamExt};

    use onecrawl_protocol::cdp::browser_protocol::animation::EventAnimationCanceled;
    use onecrawl_protocol::cdp::CustomEvent;
    use onecrawl_browser_types::MethodType;

    use super::*;

    #[tokio::test]
    async fn event_stream() {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let mut stream = EventStream::<EventAnimationCanceled>::new(rx);

        let event = EventAnimationCanceled {
            id: "id".to_string(),
        };
        let msg: Arc<dyn Event> = Arc::new(event.clone());
        tx.send(msg).await.unwrap();
        let next = stream.next().await.unwrap();
        assert_eq!(&*next, &event);
    }

    #[tokio::test]
    async fn custom_event_stream() {
        use serde::Deserialize;

        #[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
        struct MyCustomEvent {
            name: String,
        }

        impl MethodType for MyCustomEvent {
            fn method_id() -> MethodId {
                "Custom.Event".into()
            }
        }
        impl CustomEvent for MyCustomEvent {}

        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let mut stream = EventStream::<MyCustomEvent>::new(rx);

        let event = MyCustomEvent {
            name: "my event".to_string(),
        };
        let msg: Arc<dyn Event> = Arc::new(event.clone());
        tx.send(msg).await.unwrap();
        let next = stream.next().await.unwrap();
        assert_eq!(&*next, &event);
    }

    #[tokio::test]
    async fn event_listeners() {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let mut listeners = EventListeners::default();

        let event = EventAnimationCanceled {
            id: "id".to_string(),
        };

        listeners.add_listener(EventListenerRequest {
            method: EventAnimationCanceled::method_id(),
            kind: EventAnimationCanceled::event_kind(),
            listener: tx,
        });

        listeners.start_send(event.clone());

        let mut stream = EventStream::<EventAnimationCanceled>::new(rx);

        tokio::spawn(async move {
            loop {
                std::future::poll_fn(|cx| {
                    listeners.poll(cx);
                    Poll::Pending
                })
                .await
            }
        });

        let next = stream.next().await.unwrap();
        assert_eq!(&*next, &event);
    }
}
