use super::{
    message::{ASSIGN_ID, ASSIGN_IDENTIFIER, RESET_ID, RESET_IDENTIFIER},
    MessageID, MessageIdentifier,
};
use core::sync::atomic::{AtomicU8, Ordering};

pub(crate) enum RegistryLookupResult {
    ID(MessageID),
    Unassigned,
    Unknown,
}

/// Internal data structure for managing dynamic assignments of MessageIDs. Only intended for use from within the `make_network!` macro.
#[doc(hidden)]
pub struct IdentifierRegistry<'a> {
    assignments: &'a [(AtomicU8, MessageIdentifier<'static>)],
}

impl<'a> IdentifierRegistry<'a> {
    /// Internally used ID for representing unassigned IDs (so that AtomicU8 can be used as opposed to a Option<MessageID>)
    #[doc(hidden)]
    pub const UNASSIGNED: MessageID = MessageID::MAX;

    /// List of statically allocated IDs which may not be used when assigning
    const RESERVED: &'static [MessageID] = &[RESET_ID, ASSIGN_ID];

    #[doc(hidden)]
    pub const fn new(assignments: &'a [(AtomicU8, MessageIdentifier<'static>)]) -> Self {
        Self { assignments }
    }

    #[doc(hidden)]
    pub const fn verify_message_count(count: usize) {
        assert!(
            count <= MessageID::MAX as usize - Self::RESERVED.len(),
            "maximum amount of message types exceeded while creating network"
        );
    }

    pub(crate) fn assign(&self, id: MessageID, identifier: MessageIdentifier) -> bool {
        if Self::RESERVED.contains(&id) {
            return false;
        }

        for (assigned_id, message_identifier) in self.assignments.iter() {
            if *message_identifier == identifier {
                assigned_id.store(id, Ordering::Relaxed);
                return true;
            }
        }

        return false;
    }

    /// Looks up a message ID from a message identifier
    pub(crate) fn lookup(&self, identifier: MessageIdentifier) -> RegistryLookupResult {
        if identifier == RESET_IDENTIFIER {
            RegistryLookupResult::ID(RESET_ID)
        } else if identifier == ASSIGN_IDENTIFIER {
            RegistryLookupResult::ID(ASSIGN_ID)
        } else {
            for (id, assigned_identifier) in self.assignments.iter() {
                let id = id.load(Ordering::Relaxed);
                if *assigned_identifier == identifier && id != Self::UNASSIGNED {
                    return RegistryLookupResult::ID(id);
                } else if *assigned_identifier == identifier {
                    return RegistryLookupResult::Unassigned;
                }
            }

            RegistryLookupResult::Unknown
        }
    }

    /// Looks up a message identifier from a message ID
    pub(crate) fn resolve(&self, id: MessageID) -> Option<MessageIdentifier<'static>> {
        if id == MessageID::MAX {
            None
        } else if id == RESET_ID {
            Some(RESET_IDENTIFIER)
        } else if id == ASSIGN_ID {
            Some(ASSIGN_IDENTIFIER)
        } else {
            for (assigned_id, identifier) in self.assignments.iter() {
                if assigned_id.load(Ordering::Relaxed) == id {
                    return Some(identifier);
                }
            }

            None
        }
    }

    /// Removes all previous assignments
    #[cfg(feature = "peripheral")]
    pub(crate) fn clear(&self) {
        for (id, _) in self.assignments.iter() {
            id.store(0, Ordering::Relaxed);
        }
    }

    /// Statically and locally assigns IDs to each message type.
    #[cfg(feature = "host")]
    pub(crate) fn assign_all(
        &self,
    ) -> impl Iterator<Item = (MessageIdentifier<'static>, MessageID)> + '_ {
        for (new_id, (_, identifier)) in self.assignments.iter().enumerate() {
            self.assign(new_id as u8, identifier);
        }

        self.assignments.iter().filter_map(|(id, identifier)| {
            let id = id.load(Ordering::Relaxed);

            if id == Self::UNASSIGNED {
                None
            } else {
                Some((*identifier, id))
            }
        })
    }
}
