Feature: Can retrieve and set the internal log level via the REST endpoint

    Scenario: Espio wants to find out the current log level
        Given Espio doesn't know the log level
        When Espio asks for the log level
        Then Espio gets an answer

    Scenario Outline: Espio wants to set the log level
        Given the service's current log level
        When Espio requests it changes to <level>
        Then the log level is set to <level>

        Examples:
            | level |
            | error |
            | info  |
            | debug |
            | warn  |
            | trace |
