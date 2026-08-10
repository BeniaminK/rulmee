/// Merge only fields whose keys are present in a TOML table.
///
/// Usage: `merge_fields!(items, dest.section, source.section; "key" => field, ...)`
///
/// For each `"key" => field` pair, if `items` contains `"key"`, copies
/// `source.section.field` into `dest.section.field`.
macro_rules! merge_fields {
    ($items:expr, $dest:expr, $source:expr; $($key:expr => $field:ident),+ $(,)?) => {
        $(
            if $items.contains_key($key) {
                $dest.$field = $source.$field.clone();
            }
        )+
    };
}
