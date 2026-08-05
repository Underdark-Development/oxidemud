use oxide_core as core;
use oxide_core::{get_pos_room, AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

enum Amount {
    All,
    Some(u64),
}

fn parse_amount(args: &str) -> Option<Amount> {
    let t = args.trim();
    if t.is_empty() {
        return None;
    }
    if t.eq_ignore_ascii_case("all") {
        return Some(Amount::All);
    }
    t.parse::<u64>().ok().map(Amount::Some)
}

fn has_banker(world: &World, entity: core::Entity) -> bool {
    let Some(room) = get_pos_room(world, entity) else {
        return false;
    };
    for e in core::entities_in_room(world, room) {
        if world
            .query_one::<&core::Banker>(e)
            .ok()
            .is_some_and(|mut q| q.get().is_some())
        {
            return true;
        }
    }
    false
}

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "balance",
        aliases: &["bank"],
        access: AccessLevel::Player,
        topic: "Economy",
        help: CommandHelp {
            short: "Check your bank balance",
            body: Some("Usage: balance\nShows your bank balance and gold on hand. Requires a banker nearby."),
        },
        handler: cmd_balance,
    });
    server.register_command(Command {
        name: "deposit",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Economy",
        help: CommandHelp {
            short: "Deposit gold into the bank",
            body: Some("Usage: deposit <amount|all>\nDeposits gold from your wallet into your bank account. Requires a banker nearby."),
        },
        handler: cmd_deposit,
    });
    server.register_command(Command {
        name: "withdraw",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Economy",
        help: CommandHelp {
            short: "Withdraw gold from the bank",
            body: Some("Usage: withdraw <amount|all>\nWithdraws gold from your bank account into your wallet. Requires a banker nearby."),
        },
        handler: cmd_withdraw,
    });
}

fn cmd_balance(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    if !has_banker(world, entity) {
        conn.send_line("You need a banker nearby to check your balance.");
        return;
    }

    let bank_balance = world
        .query_one::<&core::BankAccount>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .map(|b| b.0)
        .unwrap_or(0);

    let on_hand = world
        .query_one::<&core::Wallet>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .map(|w| w.gold)
        .unwrap_or(0);

    conn.send_line("--- Bank of Oxide ---");
    conn.send_line(&format!(
        "{{yellow}}Bank balance: {} gold{{/}}",
        bank_balance
    ));
    conn.send_line(&format!("{{yellow}}On hand:      {} gold{{/}}", on_hand));
}

fn cmd_deposit(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    if !has_banker(world, entity) {
        conn.send_line("You need a banker nearby to make a deposit.");
        return;
    }

    let amount = match parse_amount(args) {
        Some(a) => a,
        None => {
            conn.send_line("Deposit how much gold? Use 'deposit <amount>' or 'deposit all'.");
            return;
        }
    };

    let mut wallet = match world.query_one::<&mut core::Wallet>(entity) {
        Ok(mut q) => match q.get() {
            Some(w) => w.clone(),
            None => {
                conn.send_line("You have no money.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no money.");
            return;
        }
    };

    let deposit = match amount {
        Amount::All => wallet.gold,
        Amount::Some(n) => n,
    };

    if deposit == 0 {
        conn.send_line("You have no gold to deposit.");
        return;
    }
    if wallet.gold < deposit {
        conn.send_line(&format!("You only have {} gold to deposit.", wallet.gold));
        return;
    }

    wallet.gold -= deposit;

    let mut bank = world
        .query_one::<&mut core::BankAccount>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or_default();
    bank.0 = bank.0.saturating_add(deposit);

    let _ = world.insert(entity, (wallet, bank, core::Dirty));

    conn.send_line(&format!(
        "You deposit {{yellow}}{deposit} gold{{/}}. Your bank balance is now {{yellow}}{} gold{{/}}.",
        bank.0
    ));
}

fn cmd_withdraw(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    if !has_banker(world, entity) {
        conn.send_line("You need a banker nearby to make a withdrawal.");
        return;
    }

    let amount = match parse_amount(args) {
        Some(a) => a,
        None => {
            conn.send_line("Withdraw how much gold? Use 'withdraw <amount>' or 'withdraw all'.");
            return;
        }
    };

    let mut bank = world
        .query_one::<&mut core::BankAccount>(entity)
        .ok()
        .and_then(|mut q| q.get().copied())
        .unwrap_or_default();

    let withdrawal = match amount {
        Amount::All => bank.0,
        Amount::Some(n) => n,
    };

    if withdrawal == 0 {
        conn.send_line("You have no gold in the bank to withdraw.");
        return;
    }
    if bank.0 < withdrawal {
        conn.send_line(&format!("You only have {} gold in the bank.", bank.0));
        return;
    }

    bank.0 -= withdrawal;

    let mut wallet = match world.query_one::<&mut core::Wallet>(entity) {
        Ok(mut q) => match q.get() {
            Some(w) => w.clone(),
            None => {
                conn.send_line("You have no money.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no money.");
            return;
        }
    };
    wallet.gold = wallet.gold.saturating_add(withdrawal);

    let _ = world.insert(entity, (wallet, bank, core::Dirty));

    conn.send_line(&format!(
        "You withdraw {{yellow}}{withdrawal} gold{{/}}. Your bank balance is now {{yellow}}{} gold{{/}}.",
        bank.0
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_helpers::{test_player, test_world};
    use oxide_core as core;

    fn spawn_banker(world: &mut World, room: core::Entity) {
        let _ = world.spawn((
            core::Position::new(room),
            core::Name::new("Teller"),
            core::Banker,
        ));
    }

    fn wallet_gold(world: &World, entity: core::Entity) -> u64 {
        world
            .query_one::<&core::Wallet>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .map(|w| w.gold)
            .unwrap_or(0)
    }

    fn bank_gold(world: &World, entity: core::Entity) -> u64 {
        world
            .query_one::<&core::BankAccount>(entity)
            .ok()
            .and_then(|mut q| q.get().copied())
            .map(|b| b.0)
            .unwrap_or(0)
    }

    #[test]
    fn parse_amount_all() {
        assert!(matches!(parse_amount("all"), Some(Amount::All)));
        assert!(matches!(parse_amount(" ALL "), Some(Amount::All)));
        assert!(matches!(parse_amount("All"), Some(Amount::All)));
    }

    #[test]
    fn parse_amount_number() {
        assert!(matches!(parse_amount("50"), Some(Amount::Some(50))));
        assert!(matches!(parse_amount(" 0 "), Some(Amount::Some(0))));
    }

    #[test]
    fn parse_amount_invalid() {
        assert!(parse_amount("").is_none());
        assert!(parse_amount("abc").is_none());
        assert!(parse_amount("-5").is_none());
    }

    #[test]
    fn has_banker_false_without_banker() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, _conn, _registry) = test_player(&mut world, room_a);
        assert!(!has_banker(&world, player));
    }

    #[test]
    fn has_banker_true_with_banker_in_room() {
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_banker(&mut world, room_a);
        let (player, _conn, _registry) = test_player(&mut world, room_a);
        assert!(has_banker(&world, player));
    }

    #[test]
    fn deposit_requires_banker() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 100, 0), core::BankAccount::new(0)),
        );

        cmd_deposit(&mut world, &mut conn, "TestPlayer", "all", &registry);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("banker nearby")));
        assert_eq!(wallet_gold(&world, player), 100);
        assert_eq!(bank_gold(&world, player), 0);
    }

    #[test]
    fn deposit_moves_gold_to_bank() {
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_banker(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 100, 0), core::BankAccount::new(0)),
        );

        cmd_deposit(&mut world, &mut conn, "TestPlayer", "all", &registry);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("100 gold")));
        assert_eq!(wallet_gold(&world, player), 0);
        assert_eq!(bank_gold(&world, player), 100);
        assert!(world
            .query_one::<&core::Dirty>(player)
            .ok()
            .is_some_and(|mut q| q.get().is_some()));
    }

    #[test]
    fn deposit_more_than_wallet_is_rejected() {
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_banker(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 50, 0), core::BankAccount::new(0)),
        );

        cmd_deposit(&mut world, &mut conn, "TestPlayer", "100", &registry);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("only have 50 gold")));
        assert_eq!(wallet_gold(&world, player), 50);
        assert_eq!(bank_gold(&world, player), 0);
    }

    #[test]
    fn withdraw_requires_banker() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 0, 0), core::BankAccount::new(100)),
        );

        cmd_withdraw(&mut world, &mut conn, "TestPlayer", "all", &registry);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("banker nearby")));
        assert_eq!(bank_gold(&world, player), 100);
    }

    #[test]
    fn withdraw_moves_gold_to_wallet() {
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_banker(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 0, 0), core::BankAccount::new(100)),
        );

        cmd_withdraw(&mut world, &mut conn, "TestPlayer", "all", &registry);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("100 gold")));
        assert_eq!(wallet_gold(&world, player), 100);
        assert_eq!(bank_gold(&world, player), 0);
    }

    #[test]
    fn withdraw_more_than_balance_is_rejected() {
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_banker(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 0, 0), core::BankAccount::new(50)),
        );

        cmd_withdraw(&mut world, &mut conn, "TestPlayer", "100", &registry);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("only have 50 gold")));
        assert_eq!(wallet_gold(&world, player), 0);
        assert_eq!(bank_gold(&world, player), 50);
    }

    #[test]
    fn balance_shows_bank_and_wallet() {
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_banker(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 25, 0), core::BankAccount::new(75)),
        );

        cmd_balance(&mut world, &mut conn, "TestPlayer", "", &registry);
        let lines = conn.take_lines();
        let joined = lines.join("\n");
        assert!(joined.contains("75 gold"), "output: {joined}");
        assert!(joined.contains("25 gold"), "output: {joined}");
    }
}
