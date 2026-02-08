-- Rebrand the default agent name away from "Grail".
-- Keep existing custom names as-is.
UPDATE settings
SET agent_name = 'MicroEmployee'
WHERE agent_name = 'Grail';
