Feature: Can retrieve API version info via REST requests

    Scenario: Big wants to find out the build information for the service
        Given Big doesn't know the API build info
        When Big asks for the API info
        Then Big gets an answer

