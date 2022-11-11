use crate::payment::Payment;
use crate::time::Time;

use std::collections::BTreeMap;
use std::collections::VecDeque;

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum EventType {
    ScheduledPayment { payment: Payment },
}

pub struct EventQueue {
    events: BTreeMap<Time, VecDeque<EventType>>,
    last_tick: Time,
}

enum MessageType {
    /// Offer an HTLC to another node
    UpdateAddHtlc,
    RevokeAndAck,
    CommitmentSigned,
}

impl EventQueue {
    pub fn new() -> Self {
        let events = BTreeMap::new();
        let last_tick = Time::from_millis(0.0);
        EventQueue { events, last_tick }
    }

    /// Schedules a new event at a specific simtime.
    pub(crate) fn schedule(&mut self, delay: Time, event: EventType) {
        let time = self.now() + delay;
        let result = self.events.get_mut(&time);
        match result {
            Some(event_list) => {
                event_list.push_back(event);
            }
            None => {
                let mut event_list = VecDeque::new();
                event_list.push_back(event);
                self.events.insert(time, event_list);
            }
        }
    }

    /// Returns the next event and removes it from the event queue
    pub(crate) fn next(&mut self) -> Option<EventType> {
        let mut tick_done = false;
        let mut result = None;

        // get iterator for event_list on tick t
        if let Some((t, event_list)) = self.events.iter_mut().next() {
            self.last_tick = *t;

            result = event_list.pop_front();

            if event_list.is_empty() {
                tick_done = true;
            }
        }

        if tick_done {
            self.events.remove(&self.last_tick);
        }

        result
    }

    pub(crate) fn now(&self) -> Time {
        self.last_tick
    }

    pub(crate) fn peek_next(&self) -> Option<Time> {
        let next = if let Some((t, _)) = self.events.iter().next() {
            Some(*t)
        } else {
            None
        };
        next
    }

    pub(crate) fn queue_length(&self) -> usize {
        self.events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn eventqueue_schedule_works() {
        let mut eq = EventQueue::new();

        let payment = Payment::default();
        let e = EventType::ScheduledPayment { payment };

        let t = Time::from_secs(0.0);

        eq.schedule(t, e.clone());
        eq.schedule(t, e.clone());
        eq.schedule(t, e.clone());

        let mut res = eq.next();
        assert!(res.is_some());
        if let Some(e_res) = res {
            assert_eq!(e_res, e.clone());
        }

        res = eq.next();
        assert!(res.is_some());
        if let Some(e_res) = res {
            assert_eq!(e_res, e.clone());
        }

        res = eq.next();
        assert!(res.is_some());
        if let Some(e_res) = res {
            assert_eq!(e_res, e.clone());
        }

        res = eq.next();
        assert!(res.is_none());

        res = eq.next();
        assert!(res.is_none());

        res = eq.next();
        assert!(res.is_none());
    }

    #[test]
    fn eventqueue_earlier_later_events_work() {
        let mut queue = EventQueue::new();
        let e0 = EventType::ScheduledPayment {
            payment: Payment {
                payment_id: 0,
                ..Default::default()
            },
        };
        let e1 = EventType::ScheduledPayment {
            payment: Payment {
                payment_id: 1,
                ..Default::default()
            },
        };
        let e2 = EventType::ScheduledPayment {
            payment: Payment {
                payment_id: 2,
                ..Default::default()
            },
        };

        let t0 = Time::from_secs(0.0);
        let t1 = Time::from_secs(23.0);
        let t2 = Time::from_secs(10.0);

        queue.schedule(t0, e0.clone());
        queue.schedule(t1, e1.clone());
        queue.schedule(t2, e2.clone());

        let mut res = queue.next();
        assert!(res.is_some());
        if let Some(e_res) = res {
            assert_eq!(e_res, e0);
        }
        res = queue.next();
        assert!(res.is_some());
        if let Some(e_res) = res {
            assert_eq!(e_res, e2);
        }
        res = queue.next();
        assert!(res.is_some());
        if let Some(e_res) = res {
            assert_eq!(e_res, e1);
        }
        assert!(queue.next().is_none());
        assert_eq!(queue.now(), Time::from_secs(23.0));
    }

    #[test]
    fn eventqueue_queued_times_work() {
        let mut rng = rand::thread_rng();
        let mut eq = EventQueue::new();

        let e = EventType::ScheduledPayment {
            payment: Payment {
                payment_id: 2,
                ..Default::default()
            },
        };
        let mut times = Vec::new();
        for _ in 1..100 {
            let rand_time: f32 = rng.gen_range(0.0..u64::max_value() as f32) / 1000.0;
            times.push(rand_time);

            let t = Time::from_millis(rand_time);
            eq.schedule(t, e.clone());
            println!("Scheduled for rand_time: {}, Time: {}", rand_time, t);
        }

        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        times.reverse();

        while let Some(ev) = eq.next() {
            assert_eq!(ev, e);

            let now: f32 = eq.now().as_millis();
            let next_time = times.pop().unwrap().floor();

            println!("Running at {}, Should be: {}", now, next_time);
            assert_eq!(now, next_time);
        }
        assert!(times.is_empty());
        assert!(eq.next().is_none());
    }
}
