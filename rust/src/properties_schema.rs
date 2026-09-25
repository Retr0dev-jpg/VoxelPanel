// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Known `server.properties` keys: type, range, default, group and description.

use crate::api::types::{PropertyGroup, PropertyKind, PropertySchema};

struct Spec {
    key: &'static str,
    kind: PropertyKind,
    group: PropertyGroup,
    default: &'static str,
    range: Option<(i64, i64)>,
    options: &'static [&'static str],
    it: &'static str,
    en: &'static str,
}

#[allow(clippy::too_many_arguments)]
const fn spec(
    key: &'static str,
    kind: PropertyKind,
    group: PropertyGroup,
    default: &'static str,
    range: Option<(i64, i64)>,
    options: &'static [&'static str],
    it: &'static str,
    en: &'static str,
) -> Spec {
    Spec { key, kind, group, default, range, options, it, en }
}

use PropertyGroup as G;
use PropertyKind as K;

const SPECS: &[Spec] = &[
    spec("motd", K::Text, G::General, "A Minecraft Server", None, &[], "Messaggio mostrato nell'elenco dei server. Supporta i codici colore §.", "Message shown in the server list. Supports § color codes."),
    spec("max-players", K::Integer, G::General, "20", Some((1, 100_000)), &[], "Numero massimo di giocatori collegati insieme.", "Maximum number of players online at once."),
    spec("server-port", K::Integer, G::General, "25565", Some((1, 65_535)), &[], "Porta TCP su cui il server accetta le connessioni.", "TCP port the server listens on."),
    spec("server-ip", K::Text, G::General, "", None, &[], "Indirizzo a cui legarsi. Vuoto: tutte le interfacce.", "Address to bind to. Empty: all interfaces."),
    spec("online-mode", K::Boolean, G::General, "true", None, &[], "Verifica gli account con Mojang. Disattivalo solo dietro un proxy che autentica.", "Verifies accounts with Mojang. Disable only behind an authenticating proxy."),
    spec("white-list", K::Boolean, G::General, "false", None, &[], "Solo i giocatori in whitelist possono entrare.", "Only whitelisted players can join."),
    spec("enforce-whitelist", K::Boolean, G::General, "false", None, &[], "Espelle i giocatori rimossi dalla whitelist mentre sono online.", "Kicks players removed from the whitelist while online."),
    spec("hide-online-players", K::Boolean, G::General, "false", None, &[], "Nasconde l'elenco dei giocatori nella risposta di stato.", "Hides the player list from status responses."),
    spec("gamemode", K::Choice, G::Gameplay, "survival", None, &["survival", "creative", "adventure", "spectator"], "Modalità di gioco dei nuovi giocatori.", "Game mode for new players."),
    spec("force-gamemode", K::Boolean, G::Gameplay, "false", None, &[], "Riporta i giocatori alla modalità predefinita a ogni ingresso.", "Resets players to the default game mode on join."),
    spec("difficulty", K::Choice, G::Gameplay, "easy", None, &["peaceful", "easy", "normal", "hard"], "Difficoltà del mondo.", "World difficulty."),
    spec("hardcore", K::Boolean, G::Gameplay, "false", None, &[], "Difficoltà massima e bando alla morte.", "Hardest difficulty and ban on death."),
    spec("pvp", K::Boolean, G::Gameplay, "true", None, &[], "I giocatori possono ferirsi a vicenda.", "Players can damage each other."),
    spec("allow-flight", K::Boolean, G::Gameplay, "false", None, &[], "Non espelle chi vola in sopravvivenza (utile con alcune mod).", "Does not kick flying players in survival (needed by some mods)."),
    spec("spawn-protection", K::Integer, G::Gameplay, "16", Some((0, 999)), &[], "Raggio intorno allo spawn modificabile solo dagli operatori.", "Radius around spawn that only operators can edit."),
    spec("spawn-monsters", K::Boolean, G::Gameplay, "true", None, &[], "Generazione dei mostri ostili.", "Hostile mob spawning."),
    spec("allow-nether", K::Boolean, G::Gameplay, "true", None, &[], "Permette di andare nel Nether.", "Allows travelling to the Nether."),
    spec("enable-command-block", K::Boolean, G::Gameplay, "false", None, &[], "Abilita i blocchi comandi.", "Enables command blocks."),
    spec("player-idle-timeout", K::Integer, G::Gameplay, "0", Some((0, 100_000)), &[], "Minuti di inattività prima dell'espulsione. 0: mai.", "Idle minutes before a kick. 0: never."),
    spec("level-name", K::Text, G::World, "world", None, &[], "Cartella del mondo attivo.", "Folder of the active world."),
    spec("level-seed", K::Text, G::World, "", None, &[], "Seed per i nuovi mondi. Vuoto: casuale.", "Seed for new worlds. Empty: random."),
    spec("level-type", K::Choice, G::World, "minecraft\\:normal", None, &["minecraft\\:normal", "minecraft\\:flat", "minecraft\\:large_biomes", "minecraft\\:amplified", "minecraft\\:single_biome_surface"], "Tipo di generazione dei nuovi mondi.", "Generation type for new worlds."),
    spec("generate-structures", K::Boolean, G::World, "true", None, &[], "Genera villaggi, templi e altre strutture.", "Generates villages, temples and other structures."),
    spec("generator-settings", K::Text, G::World, "{}", None, &[], "Impostazioni JSON del generatore (mondi piatti).", "Generator JSON settings (flat worlds)."),
    spec("max-world-size", K::Integer, G::World, "29999984", Some((1, 29_999_984)), &[], "Raggio massimo del bordo del mondo.", "Maximum world border radius."),
    spec("view-distance", K::Integer, G::World, "10", Some((3, 32)), &[], "Chunk inviati ai giocatori. Valori alti pesano molto.", "Chunks sent to players. High values are expensive."),
    spec("simulation-distance", K::Integer, G::World, "10", Some((3, 32)), &[], "Chunk in cui entità e redstone vengono aggiornate.", "Chunks where entities and redstone tick."),
    spec("network-compression-threshold", K::Integer, G::Network, "256", Some((-1, 65_535)), &[], "Dimensione minima dei pacchetti compressi. -1 disattiva.", "Minimum packet size to compress. -1 disables."),
    spec("rate-limit", K::Integer, G::Network, "0", Some((0, 100_000)), &[], "Pacchetti al secondo prima dell'espulsione. 0: nessun limite.", "Packets per second before a kick. 0: no limit."),
    spec("prevent-proxy-connections", K::Boolean, G::Network, "false", None, &[], "Blocca chi si collega da un indirizzo diverso da quello autenticato.", "Blocks players connecting from a different address than the authenticated one."),
    spec("use-native-transport", K::Boolean, G::Network, "true", None, &[], "Usa epoll su Linux per prestazioni migliori.", "Uses epoll on Linux for better performance."),
    spec("accepts-transfers", K::Boolean, G::Network, "false", None, &[], "Accetta giocatori trasferiti da un altro server.", "Accepts players transferred from another server."),
    spec("enforce-secure-profile", K::Boolean, G::Network, "true", None, &[], "Richiede chiavi di chat firmate da Mojang.", "Requires Mojang-signed chat keys."),
    spec("enable-status", K::Boolean, G::Network, "true", None, &[], "Risponde alle richieste di stato dell'elenco server.", "Answers server list status requests."),
    spec("max-tick-time", K::Integer, G::Performance, "60000", Some((-1, i64::MAX)), &[], "Millisecondi di un tick oltre i quali il watchdog ferma il server. -1 disattiva.", "Tick milliseconds before the watchdog stops the server. -1 disables."),
    spec("sync-chunk-writes", K::Boolean, G::Performance, "true", None, &[], "Scrive i chunk in modo sincrono: più sicuro, più lento.", "Writes chunks synchronously: safer, slower."),
    spec("entity-broadcast-range-percentage", K::Integer, G::Performance, "100", Some((10, 1000)), &[], "Distanza a cui le entità vengono inviate ai giocatori, in percentuale.", "Distance entities are sent to players, as a percentage."),
    spec("max-chained-neighbor-updates", K::Integer, G::Performance, "1000000", Some((-1, i64::MAX)), &[], "Aggiornamenti di blocchi a catena prima di fermarsi.", "Chained block updates before stopping."),
    spec("region-file-compression", K::Choice, G::Performance, "deflate", None, &["deflate", "lz4", "none"], "Compressione dei file di regione.", "Region file compression."),
    spec("pause-when-empty-seconds", K::Integer, G::Performance, "60", Some((-1, i64::MAX)), &[], "Secondi senza giocatori prima di mettere in pausa il mondo.", "Seconds without players before pausing the world."),
    spec("op-permission-level", K::Integer, G::Administration, "4", Some((0, 4)), &[], "Livello di permessi assegnato ai nuovi operatori.", "Permission level given to new operators."),
    spec("function-permission-level", K::Integer, G::Administration, "2", Some((1, 4)), &[], "Livello di permessi delle funzioni dei datapack.", "Permission level of datapack functions."),
    spec("broadcast-console-to-ops", K::Boolean, G::Administration, "true", None, &[], "Mostra agli operatori i comandi eseguiti dalla console.", "Shows console commands to operators."),
    spec("broadcast-rcon-to-ops", K::Boolean, G::Administration, "true", None, &[], "Mostra agli operatori i comandi eseguiti via RCON.", "Shows RCON commands to operators."),
    spec("log-ips", K::Boolean, G::Administration, "true", None, &[], "Registra gli indirizzi IP dei giocatori nei log.", "Logs player IP addresses."),
    spec("enable-jmx-monitoring", K::Boolean, G::Administration, "false", None, &[], "Espone le metriche via JMX.", "Exposes metrics through JMX."),
    spec("enable-query", K::Boolean, G::QueryRcon, "false", None, &[], "Abilita il protocollo GameSpy4 Query.", "Enables the GameSpy4 query protocol."),
    spec("query.port", K::Integer, G::QueryRcon, "25565", Some((1, 65_535)), &[], "Porta UDP del protocollo Query.", "UDP port of the query protocol."),
    spec("enable-rcon", K::Boolean, G::QueryRcon, "false", None, &[], "Abilita la console remota RCON.", "Enables the RCON remote console."),
    spec("rcon.port", K::Integer, G::QueryRcon, "25575", Some((1, 65_535)), &[], "Porta TCP di RCON.", "RCON TCP port."),
    spec("rcon.password", K::Secret, G::QueryRcon, "", None, &[], "Password di RCON. Non condividerla.", "RCON password. Do not share it."),
    spec("resource-pack", K::Text, G::ResourcePack, "", None, &[], "URL del resource pack da proporre ai giocatori.", "URL of the resource pack offered to players."),
    spec("resource-pack-sha1", K::Text, G::ResourcePack, "", None, &[], "SHA-1 del resource pack, per la verifica.", "SHA-1 of the resource pack, for verification."),
    spec("resource-pack-id", K::Text, G::ResourcePack, "", None, &[], "UUID del resource pack.", "UUID of the resource pack."),
    spec("resource-pack-prompt", K::Text, G::ResourcePack, "", None, &[], "Messaggio mostrato quando viene proposto il pack.", "Message shown when the pack is offered."),
    spec("require-resource-pack", K::Boolean, G::ResourcePack, "false", None, &[], "Espelle chi rifiuta il resource pack.", "Kicks players who decline the resource pack."),
    spec("initial-enabled-packs", K::Text, G::ResourcePack, "vanilla", None, &[], "Datapack attivi nei nuovi mondi.", "Datapacks enabled in new worlds."),
    spec("initial-disabled-packs", K::Text, G::ResourcePack, "", None, &[], "Datapack disattivati nei nuovi mondi.", "Datapacks disabled in new worlds."),
];

pub fn schema() -> Vec<PropertySchema> {
    SPECS
        .iter()
        .map(|spec| PropertySchema {
            key: spec.key.to_string(),
            kind: spec.kind,
            group: spec.group,
            default_value: spec.default.to_string(),
            min: spec.range.map(|range| range.0),
            max: spec.range.map(|range| range.1),
            options: spec.options.iter().map(|option| option.to_string()).collect(),
            description_it: spec.it.to_string(),
            description_en: spec.en.to_string(),
        })
        .collect()
}

/// Validates a value against the schema; unknown keys are accepted as free text.
pub fn validate(key: &str, value: &str) -> Result<(), String> {
    let Some(spec) = SPECS.iter().find(|spec| spec.key == key) else {
        return Ok(());
    };
    match spec.kind {
        K::Boolean if !matches!(value, "true" | "false") => Err(format!("{key}: usa true o false")),
        K::Integer => {
            let number: i64 = value.trim().parse().map_err(|_| format!("{key}: deve essere un numero intero"))?;
            match spec.range {
                Some((min, max)) if !(min..=max).contains(&number) => Err(format!("{key}: valore fuori intervallo ({min}–{max})")),
                _ => Ok(()),
            }
        }
        // Older versions use other level types (e.g. `default`): only these enums are strict.
        K::Choice if matches!(key, "difficulty" | "gamemode") && !spec.options.contains(&value) => Err(format!("{key}: valore non valido")),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_known_keys() {
        assert!(validate("pvp", "true").is_ok());
        assert!(validate("pvp", "yes").is_err());
        assert!(validate("view-distance", "40").is_err());
        assert!(validate("view-distance", "12").is_ok());
        assert!(validate("difficulty", "nightmare").is_err());
        assert!(validate("level-type", "default").is_ok());
        assert!(validate("custom-plugin-key", "anything").is_ok());
    }

    #[test]
    fn keys_are_unique() {
        let mut keys: Vec<&str> = SPECS.iter().map(|spec| spec.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), SPECS.len());
    }
}
