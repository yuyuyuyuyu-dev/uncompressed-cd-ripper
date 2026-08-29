use super::*;
use std::cell::RefCell;

#[derive(Default)]
struct FakeApi {
    posted: RefCell<Vec<String>>,
}

impl ReportApi for FakeApi {
    fn post(&self, body: &str) -> Result<(), String> {
        self.posted.borrow_mut().push(body.to_owned());

        Ok(())
    }
}

fn report() -> ErrorReport {
    ErrorReport {
        event_id: "fc6d8c0c43fc4630ad850ee518f1b9d0".to_owned(),
        timestamp: "2026-08-12T09:00:00Z".to_owned(),
        platform: "javascript".to_owned(),
        release: "uncompressed-cd-ripper@0.1.0".to_owned(),
        exception: Exceptions {
            values: vec![ThrownError {
                kind: "TypeError".to_owned(),
                value: "cannot read properties of undefined".to_owned(),
            }],
        },
        breadcrumbs: Breadcrumbs {
            values: vec![Breadcrumb {
                timestamp: "2026-08-12T08:59:58.750Z".to_owned(),
                category: "ripping".to_owned(),
                message: "track 3 was started".to_owned(),
            }],
        },
        contexts: Contexts {
            os: OperatingSystem {
                name: "macOS".to_owned(),
                version: "26.6.1".to_owned(),
            },
            device: Device {
                arch: "aarch64".to_owned(),
            },
        },
        tags: Tags {
            architecture: "aarch64".to_owned(),
        },
        extra: Extra {
            stacktrace: "at rip (index.js:1:1)".to_owned(),
            component_stack: "at TrackListing".to_owned(),
            comment: "it stopped on the third track".to_owned(),
        },
    }
}

#[test]
fn should_hand_the_error_report_it_was_given_to_the_api_unchanged() {
    // Arrange
    let api = FakeApi::default();
    let report = report();

    // Act
    send(&report, &api).expect("the fake accepts everything");

    // Assert
    let expected = serde_json::to_string(&report).expect("the report serialises");
    assert_eq!(api.posted.borrow().as_slice(), [expected]);
}
