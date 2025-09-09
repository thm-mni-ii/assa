pub const TABLES: &str = "SELECT c.table_name as name,
       json_agg(
         json_build_object(
           'name', column_name,
           'isNullable', is_nullable::boolean,
           'udtName', udt_name
         ) ORDER BY ordinal_position
       ) as json
FROM information_schema.columns as c
JOIN information_schema.tables as t
  ON c.table_name = t.table_name AND c.table_schema = t.table_schema
WHERE c.table_schema = 'public' AND t.table_type != 'VIEW'
GROUP BY c.table_name, t.table_type;";

pub const CONSTRAINTS: &str = "SELECT constrains.table_name as table,
       json_agg(
         json_build_object(
           'columnName', constrains.column_name,
           'name', constrains.constraint_name,
           'type', constrains.constraint_type,
           'checkClause', constrains.check_clause
         )
       ) as json
FROM (
    SELECT tc.table_name, kcu.column_name, kcu.constraint_name, tc.constraint_type, NULL as check_clause
    FROM information_schema.KEY_COLUMN_USAGE as kcu
    JOIN information_schema.table_constraints as tc ON tc.constraint_name = kcu.constraint_name
    WHERE tc.table_schema = 'public'
    UNION
    SELECT tc.table_name, SUBSTRING(cc.check_clause from '(?:^|(?:\\.\\s))(\\w+)'), tc.constraint_name, tc.constraint_type, cc.check_clause
    FROM information_schema.table_constraints as tc
    JOIN information_schema.check_constraints as cc ON cc.constraint_name = tc.constraint_name
        AND constraint_type = 'CHECK'
    WHERE tc.table_schema = 'public'
) as constrains
GROUP BY constrains.table_name;";

pub const VIEWS: &str = "SELECT table_name as table, view_definition as definition
FROM information_schema.views
WHERE table_schema = 'public';";

pub const ROUTINES: &str = "SELECT DISTINCT ON (oid)
       routine_name as name,
       routine_type as type,
       routine_definition as definition,
       pg_catalog.pg_get_function_identity_arguments(p.oid) AS parameters
FROM information_schema.routines i
JOIN pg_catalog.pg_proc p ON i.routine_name = p.proname
WHERE routine_schema = 'public';";

pub const TRIGGERS: &str = "SELECT trigger_name as name,
       event_object_table as objectTable,
       json_agg(event_manipulation) as json,
       action_statement as statement,
       action_orientation as orientation,
       action_timing as timing
FROM information_schema.triggers
WHERE trigger_schema = 'public'
GROUP BY trigger_name, action_statement, action_orientation, action_timing, event_object_table;";
