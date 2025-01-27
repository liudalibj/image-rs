// Copyright (c) 2022 Alibaba Cloud
// Copyright (c) 2022 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

//! Test for signature verification.

use common::KBC;
use image_rs::image::ImageClient;
use rstest::rstest;
use serial_test::serial;
use strum_macros::{Display, EnumString};

mod common;

/// Name of different signing schemes.
#[derive(EnumString, Display, Debug, PartialEq)]
pub enum SigningName {
    #[strum(serialize = "Simple Signing")]
    SimpleSigning,
    #[strum(serialize = "None")]
    None,
    #[strum(serialize = "Cosign")]
    Cosign,
}

struct TestItem<'a, 'b> {
    image_ref: &'a str,
    allow: bool,
    signing_scheme: SigningName,
    description: &'b str,
}

/// Four test cases.
const TESTS: [TestItem; 6] = [
    TestItem {
        image_ref: "quay.io/prometheus/busybox:latest",
        allow: true,
        signing_scheme: SigningName::None,
        description: "Allow pulling an unencrypted unsigned image from an unprotected registry.",
    },
    TestItem {
        image_ref: "quay.io/kata-containers/confidential-containers:signed",
        allow: true,
        signing_scheme: SigningName::SimpleSigning,
        description: "Allow pulling a unencrypted signed image from a protected registry.",
    },
    TestItem {
        image_ref: "quay.io/kata-containers/confidential-containers:unsigned",
        allow: false,
        signing_scheme: SigningName::None,
        description: "Deny pulling an unencrypted unsigned image from a protected registry.",
    },
    TestItem {
        image_ref: "quay.io/kata-containers/confidential-containers:other_signed",
        allow: false,
        signing_scheme: SigningName::SimpleSigning,
        description: "Deny pulling an unencrypted signed image with an unknown signature",
    },
    TestItem {
        image_ref: "quay.io/kata-containers/confidential-containers:cosign-signed",
        allow: true,
        signing_scheme: SigningName::Cosign,
        description: "Allow pulling an unencrypted signed image with cosign-signed signature",
    },
    TestItem {
        image_ref: "quay.io/kata-containers/confidential-containers:cosign-signed-key2",
        allow: false,
        signing_scheme: SigningName::Cosign,
        description: "Deny pulling an unencrypted signed image by cosign using a wrong public key",
    },
];

#[rstest]
#[trace]
#[case(KBC::Sample)]
#[case(KBC::OfflineFs)]
#[tokio::test]
#[serial]
async fn signature_verification_one_kbc(#[case] kbc: KBC) {
    kbc.prepare_test();
    // Init AA
    let mut aa = common::start_attestation_agent().expect("Failed to start attestation agent!");

    // AA parameter
    let aa_parameters = kbc.aa_parameter();

    for test in &TESTS {
        // clean former test files, which will help to test
        // a full interaction with sample-kbc.
        common::clean_configs()
            .await
            .expect("Delete configs failed.");

        // Init tempdirs
        let work_dir = tempfile::tempdir().unwrap();
        std::env::set_var("CC_IMAGE_WORK_DIR", &work_dir.path());

        // a new client for every pulling, avoid effection
        // of cache of old client.
        let mut image_client = ImageClient::default();

        // enable signature verification
        image_client.config.security_validate = true;

        let bundle_dir = tempfile::tempdir().unwrap();

        let res = image_client
            .pull_image(
                test.image_ref,
                bundle_dir.path(),
                &None,
                &Some(&aa_parameters),
            )
            .await;
        assert_eq!(
            res.is_ok(),
            test.allow,
            "Test: {}, Signing scheme: {}, {:?}",
            test.description,
            test.signing_scheme.to_string(),
            res
        );
    }

    // kill AA when the test is finished
    aa.kill().expect("Failed to stop attestation agent!");
    kbc.clean();
}
