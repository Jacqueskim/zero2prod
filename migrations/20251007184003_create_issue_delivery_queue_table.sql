-- Add migration script here
CREATE TABLE issue_delivery_queue (
    newsletter_issue_id uuid NOT NULL
        REFERENCES mewsletter_issues (newsletter_issue_id),
    subscriber_email TEXT NOT NULL
        PRIMARY KEY (newsletter_issue_id, subscriber_email)
)