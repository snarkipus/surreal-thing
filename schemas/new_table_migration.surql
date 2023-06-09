DEFINE TABLE registry SCHEMAFULL;

DEFINE FIELD registration ON registry TYPE number ASSERT $value != NONE;
DEFINE INDEX registration ON TABLE registry COLUMNS registration UNIQUE;