INSERT INTO "users" ("email",
                     "password",
                     "admin",
                     "email_verified")
VALUES ('admin@system.lynx',
        '$argon2id$v=19$m=19456,t=2,p=1$+nwcLE5G8SdTFIaiam3thA$KrKIjjR/eDHW51j+tBGgvT/G2/9nMuM9vh17ercBoo8',
        true, true);