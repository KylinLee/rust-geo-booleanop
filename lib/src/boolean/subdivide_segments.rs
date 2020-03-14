use super::compare_segments::compare_segments;
use super::compute_fields::compute_fields;
use super::helper::Float;
use super::possible_intersection::possible_intersection;
use super::sweep_event::SweepEvent;
use super::Operation;
use crate::splay::SplaySet;
use geo_types::Rect;
use std::collections::BinaryHeap;
use std::rc::Rc;

#[cfg(feature = "debug-booleanop")]
use super::sweep_event::JsonDebug;

pub fn subdivide<F>(
    event_queue: &mut BinaryHeap<Rc<SweepEvent<F>>>,
    sbbox: &Rect<F>,
    cbbox: &Rect<F>,
    operation: Operation,
) -> Vec<Rc<SweepEvent<F>>>
where
    F: Float,
{
    let mut sweep_line = SplaySet::<Rc<SweepEvent<F>>, _>::new(compare_segments);
    let mut sorted_events: Vec<Rc<SweepEvent<F>>> = Vec::new();
    let rightbound = sbbox.max.x.min(cbbox.max.x);

    while let Some(event) = event_queue.pop() {
        #[cfg(feature = "debug-booleanop")]
        {
            println!("\n{{\"processEvent\": {}}}", event.to_json_debug());
        }
        sorted_events.push(event.clone());

        if operation == Operation::Intersection && event.point.x > rightbound
            || operation == Operation::Difference && event.point.x > sbbox.max.x
        {
            break;
        }

        if event.is_left() {
            sweep_line.insert(event.clone());

            let maybe_prev = sweep_line.prev(&event);
            let maybe_next = sweep_line.next(&event);

            compute_fields(&event, maybe_prev, operation);

            if let Some(next) = maybe_next {
                #[cfg(feature = "debug-booleanop")]
                {
                    println!("{{\"seNextEvent\": {}}}", next.to_json_debug());
                }
                if possible_intersection(&event, &next, event_queue) == 2 {
                    // Recompute fields for current segment and the one above (in bottom to top order)
                    compute_fields(&event, maybe_prev, operation);
                    compute_fields(&next, Some(&event), operation);
                }
            }

            if let Some(prev) = maybe_prev {
                #[cfg(feature = "debug-booleanop")]
                {
                    println!("{{\"sePrevEvent\": {}}}", prev.to_json_debug());
                }
                if possible_intersection(&prev, &event, event_queue) == 2 {
                    let maybe_prev_prev = sweep_line.prev(&prev);
                    // Recompute fields for current segment and the one below (in bottom to top order)
                    compute_fields(&prev, maybe_prev_prev, operation);
                    compute_fields(&event, Some(prev), operation);
                }
            }
        } else if let Some(other_event) = event.get_other_event() {
            // This debug assert is only true, if we compare segments in the sweep line
            // based on identity (curently), and not by value (done previously).
            debug_assert!(
                sweep_line.contains(&other_event),
                "Sweep line misses event to be removed"
            );
            if sweep_line.contains(&other_event) {
                let maybe_prev = sweep_line.prev(&other_event).cloned();
                let maybe_next = sweep_line.next(&other_event).cloned();

                if let (Some(prev), Some(next)) = (maybe_prev, maybe_next) {
                    #[cfg(feature = "debug-booleanop")]
                    {
                        println!("Possible post intersection");
                        println!("{{\"sePostNextEvent\": {}}}", next.to_json_debug());
                        println!("{{\"sePostPrevEvent\": {}}}", prev.to_json_debug());
                    }
                    possible_intersection(&prev, &next, event_queue);
                }

                #[cfg(feature = "debug-booleanop")]
                {
                    println!("{{\"removing\": {}}}", other_event.to_json_debug());
                }
                sweep_line.remove(&other_event);
            }
        }

        let mut count = 0;
        let mut sl_events = Vec::new();
        sweep_line.traverse(&mut |se| {
            count += 1;
            sl_events.push(se.clone());
        });
        debug_assert_eq!(count, sweep_line.len());
        if count > 1 {
            for i in 0 .. sl_events.len() - 1 {
                println!("{:?} {:?}", compare_segments(&sl_events[i], &sl_events[i + 1]), compare_segments(&sl_events[i + 1], &sl_events[i]));
                if compare_segments(&sl_events[i], &sl_events[i + 1]) != std::cmp::Ordering::Less {
                    println!("{{\"violating1\": {}}}", sl_events[i].to_json_debug());
                    println!("{{\"violating2\": {}}}", sl_events[i + 1].to_json_debug());
                    panic!("Sweep line order is violated");
                }
            }
        }
    }

    sorted_events
}
