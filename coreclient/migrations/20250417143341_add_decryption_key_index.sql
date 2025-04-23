-- Recreate users table to add decryption key index
DROP TABLE IF EXISTS users;

CREATE TABLE IF NOT EXISTS users (
    user_name TEXT NOT NULL PRIMARY KEY,
    decryption_key_index BLOB NOT NULL,
    display_name TEXT,
    profile_picture BLOB
);