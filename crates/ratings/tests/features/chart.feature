Feature: List of top 20 snaps
    Background:
        Given a snap with id "3Iwi803Tk3KQwyD6jFiAJdlq8MLgBIoD" gets 100 votes where 75 are upvotes
        Given 25 test snaps gets between 150 and 200 votes, where 125 to 175 are upvotes

    Scenario: Tails opens the store homepage, seeing the top snaps
        When the client fetches the top snaps
        Then the top 20 snaps are returned in the proper order

    Scenario Outline: Tails opens a few store categories, retrieving the top chart for those snaps
        When the client fetches the top snaps for <category>
        Then the top snap returned is the one with the ID "3Iwi803Tk3KQwyD6jFiAJdlq8MLgBIoD"

        Examples:
            | category    |
            | Utilities   |
            | Development |