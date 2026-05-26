use std::collections::BTreeMap;

use uuid::Uuid;

use crate::{CardRecord, ClusterMemberRecord, RetroColumnRecord};

pub(crate) fn attach_cards_to_columns(columns: &mut [RetroColumnRecord], cards: Vec<CardRecord>) {
    let mut member_cards = BTreeMap::<Uuid, Vec<ClusterMemberRecord>>::new();
    let mut top_level_cards = Vec::new();
    for card in cards {
        if let Some(parent_card_id) = card.parent_card_id {
            member_cards
                .entry(parent_card_id)
                .or_default()
                .push(ClusterMemberRecord::from(&card));
        } else {
            top_level_cards.push(card);
        }
    }
    for card in &mut top_level_cards {
        card.cluster_members = member_cards.remove(&card.id).unwrap_or_default();
    }

    for column in columns {
        column.cards = top_level_cards
            .iter()
            .filter(|card| card.column_id == column.id)
            .cloned()
            .collect();
    }
}
