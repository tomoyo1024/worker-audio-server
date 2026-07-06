pub struct SourceConfig {
    pub id: &'static str,
    pub display: &'static str,
}

pub static SOURCES: &[SourceConfig] = &[
    SourceConfig {
        id: "nhk16",
        display: "NHK16 %s",
    },
    SourceConfig {
        id: "shinmeikai8",
        display: "SMK8 %s",
    },
    SourceConfig {
        id: "daijisen",
        display: "Daijisen %s",
    },
    SourceConfig {
        id: "taas",
        display: "TAAS %s",
    },
    SourceConfig {
        id: "forvo",
        display: "Forvo (%s)",
    },
    SourceConfig {
        id: "forvo_ext",
        display: "Forvo Ext (%s)",
    },
    SourceConfig {
        id: "forvo_ext2",
        display: "Forvo Ext2 (%s)",
    },
    SourceConfig {
        id: "jpod",
        display: "Jpod101",
    },
    SourceConfig {
        id: "jpod_alternate",
        display: "JPod101 Alt",
    },
    SourceConfig {
        id: "ozk5",
        display: "OZK5 %s",
    },
    SourceConfig {
        id: "oald10",
        display: "OALD10 %s",
    },
];

pub fn display_for(source_id: &str) -> Option<&'static str> {
    SOURCES
        .iter()
        .find(|s| s.id == source_id)
        .map(|s| s.display)
}

pub fn is_known_source(source_id: &str) -> bool {
    SOURCES.iter().any(|s| s.id == source_id)
}

pub fn all_source_ids() -> Vec<&'static str> {
    SOURCES.iter().map(|s| s.id).collect()
}
