Feature: User authentication

    Scenario: The Snap Store tries to authenticate
        Given a valid client hash
        When the client attempts to authenticate
        Then the returned token is valid

    Rule: Client hashes must be exactly 64 characters long
        Scenario Outline: Eggman tries to directly rate a Snap with an unofficial client with an improper hardcoded "hash"
            Given a bad client with the hash <hash>
            When the client attempts to authenticate
            Then the authentication is rejected

            Examples:
                | hash                                                               |
                | notarealhash                                                       |
                | abcdefghijkabcdefghijkabcdefghijkabcdefghijkabcdefghijkabcdefghijk |

        Scenario: Charmy's client authenticates twice
            Given a valid client hash
            Given an authenticated client
            When that client authenticates a second time
            Then both tokens are valid
            And the hash is only in the database once
