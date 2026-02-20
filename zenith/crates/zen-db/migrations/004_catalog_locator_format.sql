-- 004_catalog_locator_format.sql
-- Breaking migration: canonicalize catalog entries to table URI locators only.

-- Normalize accidental "...symbols.lance?#symbols" and "...#symbols" rows.
UPDATE dl_data_file
SET lance_path = RTRIM(CASE
    WHEN instr(lance_path, '#') > 0 THEN substr(lance_path, 1, instr(lance_path, '#') - 1)
    ELSE lance_path
END, '?')
WHERE instr(lance_path, '.lance') > 0
  AND instr(lance_path, '#') > 0;

-- Keep only canonical table URI locators.
DELETE FROM dl_data_file
WHERE instr(lance_path, '.lance') = 0;

-- Repair duplicate historical symbols rows by keeping only the newest
-- per package/version visibility scope.
DELETE FROM dl_data_file
WHERE id IN (
    SELECT id
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                   PARTITION BY ecosystem,
                                package,
                                version,
                                visibility,
                                COALESCE(org_id, ''),
                                COALESCE(owner_sub, '')
                   ORDER BY created_at DESC, id DESC
               ) AS rn
        FROM dl_data_file
        WHERE lance_path LIKE '%/symbols.lance'
    )
    WHERE rn > 1
);
