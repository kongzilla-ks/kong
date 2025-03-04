use crate::state::*;
use crate::types::*;

#[ic_cdk::query]
pub fn get_messages(params: Option<PaginationParams>) -> MessagesPage {
    let params = params.unwrap_or(PaginationParams {
        cursor: None,
        limit: None,
    });
    
    // Convert u64 limit to usize, with bounds checking
    let limit = params.limit
        .map(|l| l.try_into().unwrap_or(DEFAULT_PAGE_SIZE))
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .min(MAX_MESSAGES_STORED);

    MESSAGE_STORE.with(|store| {
        let store = store.borrow();
        
        // Return early if there are no messages
        if store.len() == 0 {
            return MessagesPage {
                messages: Vec::new(),
                next_cursor: None,
            };
        }

        // Collect all messages
        let mut messages: Vec<Message> = store.iter().map(|(_, msg)| msg.clone()).collect();
        
        // Sort by newest first (using timestamp)
        messages.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply cursor-based pagination using timestamps
        let start_idx = match params.cursor {
            Some(cursor_timestamp) => messages
                .iter()
                .position(|msg| msg.created_at <= cursor_timestamp)
                .unwrap_or(messages.len()),
            None => 0,
        };

        // Get the page of messages
        let messages = messages
            .into_iter()
            .skip(start_idx)
            .take(limit)
            .collect::<Vec<_>>();

        // Set next cursor to the timestamp of the last message if we have more messages
        let next_cursor = if messages.len() == limit {
            messages.last().map(|msg| msg.created_at)
        } else {
            None
        };

        MessagesPage {
            messages,
            next_cursor,
        }
    })
}

#[ic_cdk::query]
pub fn get_message(id: u64) -> Option<Message> {
    MESSAGE_STORE.with(|store| {
        store.borrow().get(&id)
    })
} 