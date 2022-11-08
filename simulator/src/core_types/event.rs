use crate::time::Time;
use crate::graph::{NodeRef, EdgeRef};
use crate::payment::MessageType;

use std::collections::BTreeMap;
use std::collections::VecDeque;

#[derive(Eq,PartialEq,Debug,Clone)]
pub enum EventType {
    MessageReceived {sender: NodeRef, receiver: NodeRef, edge: EdgeRef, message: MessageType },
    ScheduledPayment {source: NodeRef, destination: NodeRef, amount: u64 },
}


pub struct EventQueue {
    events: BTreeMap<Time, VecDeque<EventType>>,
    last_tick: Time,
}

impl EventQueue {
    pub fn new() -> Self {
        let events = BTreeMap::new();
        let last_tick = Time::from_millis(0.0);
        EventQueue { events, last_tick }
    }

    // Schedules a new event at a specific simtime.
    pub fn schedule(&mut self, delay: Time, event: EventType) {
        let time = self.now() + delay;
        let result = self.events.get_mut(&time);
        match result {
            Some(event_list) => {
                event_list.push_back(event);
            },
            None => {
                let mut event_list = VecDeque::new();
                event_list.push_back(event);
                self.events.insert(time, event_list);
            },
        }
    }
    
    // Returns the next event and removes it from the event queue
    pub fn next(&mut self) -> Option<EventType> {
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

    pub fn now(&self) -> Time {
            return self.last_tick;
        //}
    }

    pub fn peek_next(&self) -> Option<Time> {
        if let Some((t, _)) = self.events.iter().next() {
            return Some(*t);
        }
        return None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::Region;
    use crate::latency::LatencyModel;
    use crate::graph::{Node, Edge};
    use std::rc::Rc;
    use std::cell::RefCell;
    use rand::Rng;
    use std::f32;

    #[test]
    fn event_traits_work() {
        let latency_model = LatencyModel::new();
        let sender_ref: NodeRef = Rc::new(RefCell::new(Node::new(0, Region::NA)));
        let receiver_ref: NodeRef = Rc::new(RefCell::new(Node::new(1, Region::EU)));
        let latency_dist = latency_model.rand_lat_dist(Region::NA, Region::EU).unwrap(); 
        let edge_ref: EdgeRef = Rc::new(RefCell::new(Edge::new(0, 0, 0, 1, latency_dist, 0, 0, 0, 0, 0, 0)));
        let e = EventType::MessageReceived{sender: sender_ref, receiver: receiver_ref, edge: edge_ref, message: MessageType::TestDummy};
        assert_eq!(e,e.clone());
    }

    #[test]
    fn eventqueue_schedule_works() {
        let mut eq = EventQueue::new();

        let latency_model = LatencyModel::new();
        let sender_ref: NodeRef = Rc::new(RefCell::new(Node::new(0, Region::NA)));
        let receiver_ref: NodeRef = Rc::new(RefCell::new(Node::new(1, Region::EU)));
        let latency_dist = latency_model.rand_lat_dist(Region::NA, Region::EU).unwrap(); 
        let edge_ref: EdgeRef = Rc::new(RefCell::new(Edge::new(0, 0, 0, 1, latency_dist, 0, 0, 0, 0, 0, 0)));
        let e = EventType::MessageReceived{sender: sender_ref, receiver: receiver_ref, edge: edge_ref, message: MessageType::TestDummy};

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
        let mut eq = EventQueue::new();

        let latency_model = LatencyModel::new();
        let sender_ref: NodeRef = Rc::new(RefCell::new(Node::new(0, Region::NA)));
        let receiver_ref: NodeRef = Rc::new(RefCell::new(Node::new(1, Region::EU)));
        let latency_dist = latency_model.rand_lat_dist(Region::NA, Region::EU).unwrap(); 
        let edge_ref: EdgeRef = Rc::new(RefCell::new(Edge::new(0, 0, 0, 1, latency_dist, 0, 0, 0, 0, 0, 0)));

        let e0 = EventType::MessageReceived{sender: sender_ref.clone(), receiver: receiver_ref.clone(), edge: edge_ref.clone(), message: MessageType::TestDummy};
        let e1 = EventType::MessageReceived{sender: sender_ref.clone(), receiver: receiver_ref.clone(), edge: edge_ref.clone(), message: MessageType::TestDummy};
        let e2 = EventType::MessageReceived{sender: sender_ref.clone(), receiver: receiver_ref.clone(), edge: edge_ref.clone(), message: MessageType::TestDummy};

        let t0 = Time::from_secs(0.0);
        let t1 = Time::from_secs(230.0);
        let t2 = Time::from_secs(100.0);

        // We schedule them in order
        eq.schedule(t0, e0.clone());
        eq.schedule(t1, e1.clone());
        eq.schedule(t2, e2.clone());

        // but should get e0, then e2, then e1
        let mut res = eq.next();
        assert!(res.is_some());
        if let Some(e_res) = res { 
            assert_eq!(e_res, e0);
        }

        res = eq.next();
        assert!(res.is_some());
        if let Some(e_res) = res { 
            assert_eq!(e_res, e2);
        }

        res = eq.next();
        assert!(res.is_some());
        if let Some(e_res) = res { 
            assert_eq!(e_res, e1);
        }

        res = eq.next();
        assert!(res.is_none());

        assert_eq!(eq.now(), Time::from_secs(230.0));
    }

    #[test]
    fn eventqueue_queued_times_work() {
        let mut rng = rand::thread_rng();
        let mut eq = EventQueue::new();

        let latency_model = LatencyModel::new();
        let sender_ref: NodeRef = Rc::new(RefCell::new(Node::new(0, Region::NA)));
        let receiver_ref: NodeRef = Rc::new(RefCell::new(Node::new(1, Region::EU)));
        let latency_dist = latency_model.rand_lat_dist(Region::NA, Region::EU).unwrap(); 
        let edge_ref: EdgeRef = Rc::new(RefCell::new(Edge::new(0, 0, 0, 1, latency_dist, 0, 0, 0, 0, 0, 0)));
        let e = EventType::MessageReceived{sender: sender_ref.clone(), receiver: receiver_ref.clone(), edge: edge_ref.clone(), message: MessageType::TestDummy};

        let mut times = Vec::new();
        for _ in 1..100 {
            let rand_time: f32 = rng.gen_range(0.0, u64::max_value() as f32) / 1000.0;
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
