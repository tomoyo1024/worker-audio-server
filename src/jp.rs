const KATAKANA_CHART: &str = "ァアィイゥウェエォオカガカ゚キギキ゚クグク゚ケゲケ゚コゴコ゚サザシジスズセゼソゾタダチヂッツヅテデトドナニヌネノハバパヒビピフブプヘベペホボポマミムメモャヤュユョヨラリルレロヮワヰヱヲンヴヵヶヽヾ";

const HIRAGANA_CHART: &str = "ぁあぃいぅうぇえぉおかがか゚きぎき゚くぐく゚けげけ゚こごこ゚さざしじすずせぜそぞただちぢっつづてでとどなにぬねのはばぱひびぴふぶぷへべぺほぼぽまみむめもゃやゅゆょよらりるれろゎわゐゑをんゔゕゖゝゞ";

pub fn katakana_to_hiragana(kana: &str) -> String {
    use std::sync::OnceLock;
    static MAP: OnceLock<std::collections::HashMap<char, char>> = OnceLock::new();
    let map = MAP.get_or_init(|| KATAKANA_CHART.chars().zip(HIRAGANA_CHART.chars()).collect());
    kana.chars().map(|c| *map.get(&c).unwrap_or(&c)).collect()
}
