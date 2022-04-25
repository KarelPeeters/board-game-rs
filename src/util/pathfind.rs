use crate::board::Board;
use internal_iterator::InternalIterator;

/// Find a list of `len` moves that when played on `start` results in `target`.
pub fn pathfind_exact_length<B: Board>(start: &B, target: &B, len: u32) -> Option<Vec<B::Move>> {
    if len == 0 || start.is_done() {
        return if start == target { Some(vec![]) } else { None };
    }

    start
        .available_moves()
        .filter_map(|mv: B::Move| {
            let next = start.clone_and_play(mv);

            pathfind_exact_length(&next, target, len - 1).map(|mut left| {
                left.insert(0, mv);
                left
            })
        })
        .next()
}
