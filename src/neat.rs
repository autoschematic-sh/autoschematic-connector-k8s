use regex::Regex;
use serde_yaml::{Mapping, Value};

/// Remove non-user-configurable fields from a Kubernetes object.
pub fn neatify_resource(v: &mut Value) {
    // Top-level must be a mapping
    let Some(obj) = v.as_mapping_mut() else { return };

    // 1) Drop entire .status
    obj.remove(&Value::from("status"));

    // 2) metadata hygiene
    if let Some(meta) = obj.get_mut(&Value::from("metadata")).and_then(Value::as_mapping_mut) {
        // Drop standard server fields
        for k in [
            "managedFields",
            "resourceVersion",
            "uid",
            "generation",
            "creationTimestamp",
            "selfLink",
        ] {
            meta.remove(&Value::from(k));
        }

        // metadata.annotations cleanup
        if let Some(ann) = meta.get_mut(&Value::from("annotations")).and_then(Value::as_mapping_mut)
        {
            let drop_exact = [
                "kubectl.kubernetes.io/last-applied-configuration",
                "deployment.kubernetes.io/revision",
                "meta.helm.sh/release-name",
                "meta.helm.sh/release-namespace",
            ];

            let re_hashish = Regex::new(r"(?i)(?:^|[./-])(checksum|hash)(?:$|[./-])").unwrap();

            // Collect keys to remove (can’t mutate while iterating)
            let mut to_del: Vec<Value> = ann
                .keys()
                .filter_map(|k| k.as_str().map(|s| s.to_string()))
                .filter(|k| {
                    drop_exact.contains(&k.as_str())
                        || re_hashish.is_match(k)
                        || k.ends_with("-hash")
                })
                .map(Value::from)
                .collect();

            // Delete them
            for k in to_del.drain(..) {
                ann.remove(&k);
            }

            // If annotations ends up empty, drop it
            if ann.is_empty() {
                meta.remove(&Value::from("annotations"));
            }
        }

        // metadata.labels cleanup (only clear obvious Helm noise)
        if let Some(lbl) = meta.get_mut(&Value::from("labels")).and_then(Value::as_mapping_mut) {
            // remove helm/chart label and managed-by=Helm
            lbl.remove(&Value::from("helm.sh/chart"));
            if let Some(v) = lbl.get(&Value::from("app.kubernetes.io/managed-by")) {
                if v.as_str() == Some("Helm") {
                    lbl.remove(&Value::from("app.kubernetes.io/managed-by"));
                }
            }
            if lbl.is_empty() {
                meta.remove(&Value::from("labels"));
            }
        }
    }

    // 3) spec.template.metadata.annotations: drop rolling-hash/checksum noise
    if let Some(spec) = obj.get_mut(&Value::from("spec")).and_then(Value::as_mapping_mut) {
        if let Some(tpl) = spec.get_mut(&Value::from("template")).and_then(Value::as_mapping_mut) {
            if let Some(tpl_meta) = tpl.get_mut(&Value::from("metadata")).and_then(Value::as_mapping_mut) {
                if let Some(tpl_ann) = tpl_meta.get_mut(&Value::from("annotations")).and_then(Value::as_mapping_mut) {
                    let re_hashish = Regex::new(r"(?i)(?:^|[./-])(checksum|hash)(?:$|[./-])").unwrap();
                    // examples often look like hooks-hash, *-config-hash, checksum/*
                    let mut to_del: Vec<Value> = tpl_ann
                        .keys()
                        .filter_map(|k| k.as_str().map(|s| s.to_string()))
                        .filter(|k| {
                            re_hashish.is_match(k)
                                || k.ends_with("-hash")
                                || k.contains("config-hash")
                                || k.starts_with("checksum/")
                        })
                        .map(Value::from)
                        .collect();
                    for k in to_del.drain(..) {
                        tpl_ann.remove(&k);
                    }
                    if tpl_ann.is_empty() {
                        tpl_meta.remove(&Value::from("annotations"));
                    }
                }
                if tpl_meta.is_empty() {
                    tpl.remove(&Value::from("metadata"));
                }
            }
        }
    }
}
