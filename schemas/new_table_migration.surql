DEFINE TABLE registry SCHEMAFULL;

DEFINE FIELD id ON registry TYPE string ASSERT $value != NONE;
DEFINE FIELD registration ON registry TYPE number ASSERT $value != NONE;
DEFINE INDEX registration ON TABLE registry COLUMNS registration UNIQUE;