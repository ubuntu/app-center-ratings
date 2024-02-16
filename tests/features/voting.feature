Feature: User voting
    Background:
        Given a Snap named "chu-chu-garden" has already accumulated 5 votes and 3 upvotes

    Scenario: Amy upvotes a snap she hasn't voted for in the past
        When Amy casts an upvote
        Then the total number of votes strictly increases
        And the ratings band monotonically increases

    Rule: Votes that a user updates do not change the total vote count

        Scenario Outline: Sonic changes his vote between downvote and upvote because "chu-chu-garden" got better/worse
            Given Sonic originally voted <original>
            When Sonic changes his vote to <after>
            Then the ratings band <direction>
            But the total number of votes stays constant

            Examples:
                | original | after    | direction               |
                | upvote   | downvote | monotonically increases |
                | downvote | upvote   | monotonically decreases |

