use std::{
    borrow::Borrow,
    fmt::{Debug, Display},
};

use serde::{Deserialize, Serialize};

pub mod game_state;
pub mod lobby;
pub mod net;
pub mod player;

pub const MAX_PLAYERS: usize = 6;

// Setup Newtype pattern for IDs
macro_rules! decl_id {
    ($name:ident) => {
        #[derive(Copy, Clone, PartialEq, Eq, Deserialize, Serialize, Hash)]
        pub struct $name(pub u32);

        impl Debug for $name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                <Self as Display>::fmt(self, f)
            }
        }
        impl Display for $name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                // Always diplay IDs in hex
                write!(f, "{:#X}", self.0)
            }
        }

        impl From<u32> for $name {
            #[inline]
            fn from(v: u32) -> Self {
                Self(v)
            }
        }
        impl From<$name> for u32 {
            #[inline]
            fn from(v: $name) -> Self {
                v.0
            }
        }

        impl Borrow<u32> for $name {
            #[inline]
            fn borrow(&self) -> &u32 {
                &self.0
            }
        }
        impl PartialEq<u32> for $name {
            #[inline]
            fn eq(&self, other: &u32) -> bool {
                self.0 == *other
            }
        }
    };
}

decl_id!(PlayerId);
decl_id!(LobbyId);
