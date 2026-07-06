pub struct QueryComponents<'a> {
    pub expression: &'a str,
    /// When `None`, reading is not checked at all.
    pub reading: Option<&'a str>,
    pub sources: &'a [String],
    pub user: &'a [String],
}

pub fn build_query(q: &QueryComponents<'_>, total_sources: usize) -> (String, Vec<String>) {
    let mut params: Vec<String> = Vec::new();

    // --- WHERE clause -------------------------------------------------------
    let mut query_where = String::new();

    let match_order: Option<&str> = if q.reading.is_none() {
        // do not check reading at all
        params.push(q.expression.to_string());
        query_where.push_str("expression = ?");
        None
    } else {
        params.push(q.expression.to_string());
        params.push(q.reading.unwrap().to_string());
        query_where.push_str("(expression = ? OR reading = ?)");
        Some(MATCH_ORDER_SQL)
    };

    // filters by sources if necessary
    if q.sources.len() != total_sources {
        let marks = question_marks(q.sources.len());
        query_where.push_str(&format!(" AND (source in ({marks}))"));
        for s in q.sources {
            params.push(s.to_string());
        }
    }

    // filters by speakers if necessary
    if !q.user.is_empty() {
        let marks = question_marks(q.user.len());
        query_where.push_str(&format!(" AND (speaker IS NULL OR speaker in ({marks}))"));
        for u in q.user {
            params.push(u.to_string());
        }
    }

    // --- ORDER BY clause ----------------------------------------------------
    let mut order_parts: Vec<String> = Vec::new();

    if let Some(match_sql) = match_order {
        order_parts.push(match_sql.to_string());
        // expression, reading, expression, reading
        params.push(q.expression.to_string());
        params.push(q.reading.unwrap().to_string());
        params.push(q.expression.to_string());
        params.push(q.reading.unwrap().to_string());
    }

    // orders by source
    order_parts.push(case_when_sql("source", q.sources.len()));
    for s in q.sources {
        params.push(s.to_string());
    }

    // orders by speakers if necessary
    if !q.user.is_empty() {
        order_parts.push(case_when_sql("speaker", q.user.len()));
        for u in q.user {
            params.push(u.to_string());
        }
    }

    let query_order = order_parts.join(",\n");

    let sql = format!("SELECT * FROM entries WHERE (\n{query_where}\n)\nORDER BY\n{query_order}");

    (sql, params)
}

const MATCH_ORDER_SQL: &str = "(CASE
                WHEN expression = ? AND reading = ? THEN 0
                WHEN expression = ? THEN 1
                WHEN reading = ? THEN 2
                ELSE 3
            END)";

/// Produce `?, ?, ..., ?` with `n` placeholders.
fn question_marks(n: usize) -> String {
    (0..n).map(|_| "?").collect::<Vec<_>>().join(",")
}

/// Produce `(CASE <col> WHEN ? THEN 0 WHEN ? THEN 1 ... END)`.
fn case_when_sql(col: &str, n: usize) -> String {
    let mut s = format!("(CASE {col} ");
    for i in 0..n {
        s.push_str(&format!("WHEN ? THEN {i} "));
    }
    s.push_str("END)");
    s
}
