base <- paste0(
  c(
    "MTD\tmzTab-version\t2.1.0-M",
    "MTD\tmzTab-ID\tR-TEST-1",
    "MTD\tmzTab-profile\tM+S",
    "MTD\ttitle\tR test fixture",
    "MTD\tdescription\tMinimal valid mzTab-M document",
    "MTD\tsample_processing[1]\t[MSIO, MSIO:0000146, centrifugation, ]",
    "MTD\tinstrument[1]-name\t[MS, MS:1000483, Thermo Fisher Scientific instrument model, ]",
    "MTD\tinstrument[1]-source\t[MS, MS:1000073, electrospray ionization, ]",
    "MTD\tinstrument[1]-analyzer[1]\t[MS, MS:1000484, orbitrap, ]",
    "MTD\tinstrument[1]-detector\t[MS, MS:1000253, electron multiplier, ]",
    "MTD\tsoftware[1]\t[MS, MS:1002879, Progenesis QI, 3.0]",
    "MTD\tcontact[1]-name\tAda Lovelace",
    "MTD\tcontact[1]-affiliation\tAnalytical Engines",
    "MTD\tcontact[1]-email\tada@example.org",
    "MTD\tquantification_method\t[MS, MS:1001834, LC-MS label-free quantitation analysis, ]",
    "MTD\tsample[1]\tSample 1",
    "MTD\tms_run[1]-location\tfile:///a.mzML",
    "MTD\tms_run[1]-instrument_ref\tinstrument[1]",
    "MTD\tms_run[1]-format\t[MS, MS:1000584, mzML format, ]",
    "MTD\tms_run[1]-id_format\t[MS, MS:1000768, Thermo nativeID format, ]",
    "MTD\tms_run[1]-scan_polarity[1]\t[MS, MS:1000130, positive scan, ]",
    "MTD\tassay[1]\tAssay 1",
    "MTD\tassay[1]-sample_ref\tsample[1]",
    "MTD\tassay[1]-ms_run_ref\tms_run[1]",
    "MTD\tstudy_variable[1]\tGroup 1",
    "MTD\tstudy_variable[1]-assay_refs\tassay[1]",
    "MTD\tstudy_variable[1]-ms_run_refs\tms_run[1]",
    "MTD\tstudy_variable[1]-description\tGroup 1",
    "MTD\tstudy_variable[1]-average_function\t[MS, MS:1002883, median, ]",
    "MTD\tstudy_variable[1]-variation_function\t[MS, MS:1002885, standard error, ]",
    "MTD\tstudy_variable_group[1]\t[STATO, STATO:0000087, independent variable, ]",
    "MTD\tstudy_variable_group[1]-description\tGroup",
    "MTD\tstudy_variable_group[1]-type\t[STATO, STATO:0000252, categorical variable, ]",
    "MTD\tstudy_variable_group[1]-datatype\txsd:string",
    "MTD\tstudy_variable_group[1]-unit\t[UO, UO:0000186, dimensionless unit, ]",
    "MTD\tstudy_variable_group[1]-study_variable_refs\tstudy_variable[1]",
    "MTD\tcv[1]-label\tMS",
    "MTD\tcv[1]-full_name\tPSI-MS",
    "MTD\tcv[1]-version\t4.1.0",
    "MTD\tcv[1]-uri\thttps://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo",
    "MTD\tcv[2]-label\tMSIO",
    "MTD\tcv[2]-full_name\tMetabolomics Standards Initiative Ontology",
    "MTD\tcv[2]-version\t1.0",
    "MTD\tcv[2]-uri\thttps://purl.obolibrary.org/obo/msio.owl",
    "MTD\tcv[3]-label\tMIRIAM",
    "MTD\tcv[3]-full_name\tMIRIAM Registry",
    "MTD\tcv[3]-version\t1.0",
    "MTD\tcv[3]-uri\thttps://identifiers.org/",
    "MTD\tcv[4]-label\tPRIDE",
    "MTD\tcv[4]-full_name\tPRIDE CV",
    "MTD\tcv[4]-version\t1.0",
    "MTD\tcv[4]-uri\thttps://www.ebi.ac.uk/ols/ontologies/pride",
    "MTD\tcv[5]-label\tSTATO",
    "MTD\tcv[5]-full_name\tSTATO",
    "MTD\tcv[5]-version\t1.4",
    "MTD\tcv[5]-uri\thttp://stato-ontology.org/",
    "MTD\tcv[6]-label\tUO",
    "MTD\tcv[6]-full_name\tUnits of Measurement Ontology",
    "MTD\tcv[6]-version\t1.0",
    "MTD\tcv[6]-uri\thttps://purl.obolibrary.org/obo/uo.owl",
    "MTD\tdatabase[1]\t[MIRIAM, MIR:00100079, HMDB, ]",
    "MTD\tdatabase[1]-prefix\tHMDB",
    "MTD\tdatabase[1]-version\t5.0",
    "MTD\tdatabase[1]-uri\thttps://hmdb.ca",
    "MTD\tsmall_molecule-quantification_unit\t[PRIDE, PRIDE:0000393, Relative quantification unit, ]",
    "MTD\tsmall_molecule-identification_reliability\t[MS, MS:1002896, compound identification confidence level, ]",
    "",
    "SMH\tSML_ID\tSMF_ID_REFS\tdatabase_identifier\tchemical_formula\tsmiles\tinchi\tchemical_name\turi\ttheoretical_neutral_mass\tadduct_ions\treliability\tbest_id_confidence_measure\tbest_id_confidence_value\tabundance_assay[1]\tabundance_study_variable[1]\tabundance_variation_study_variable[1]",
    "SML\t1\tnull\tHMDB:HMDB0000122\tC6H12O6\tnull\tnull\tGlucose\tnull\t180.0634\t[M+H]1+\t2\t[MS, MS:1002890, fragmentation score, ]\t0.95\t100.0\t100.0\t0.0",
    "SML\t2\tnull\tnull\tnull\tnull\tnull\tnull\tnull\tnull\tnull\t4\tnull\t0.5\tnull\tnull\tnull"
  ),
  collapse = "\n"
)
base <- paste0(base, "\n")

base_json <- paste0(
  '{"metadata":{"prefix":"MTD","mzTab-version":"2.1.0-M","mzTab-ID":"R-TEST-1","mzTab-profile":"M+S",',
  '"title":"R test fixture","description":"Minimal valid mzTab-M document",',
  '"contact":[{"id":1,"name":"Ada Lovelace","affiliation":"Analytical Engines","email":"ada@example.org","orcid":null}],',
  '"publication":null,"uri":null,"external_study_uri":null,',
  '"instrument":[{"id":1,"name":{"cv_label":"MS","cv_accession":"MS:1000483","name":"Thermo Fisher Scientific instrument model","value":null},',
  '"source":{"cv_label":"MS","cv_accession":"MS:1000073","name":"electrospray ionization","value":null},',
  '"analyzer":[{"id":1,"cv_label":"MS","cv_accession":"MS:1000484","name":"orbitrap","value":null}],',
  '"detector":{"cv_label":"MS","cv_accession":"MS:1000253","name":"electron multiplier","value":null}}],',
  '"quantification_method":{"cv_label":"MS","cv_accession":"MS:1001834","name":"LC-MS label-free quantitation analysis","value":null},',
  '"sample":[{"id":1,"name":"Sample 1","custom":null,"species":null,"tissue":null,"cell_type":null,"disease":null,"description":null}],',
  '"protocol":null,',
  '"sample_processing":[{"id":1,"sample_processing":[{"cv_label":"MSIO","cv_accession":"MSIO:0000146","name":"centrifugation","value":null}]}],',
  '"software":[{"id":1,"parameter":{"cv_label":"MS","cv_accession":"MS:1002879","name":"Progenesis QI","value":"3.0"},"setting":null}],',
  '"derivatization_agent":null,',
  '"ms_run":[{"id":1,"location":"file:///a.mzML","instrument_ref":1,',
  '"format":{"cv_label":"MS","cv_accession":"MS:1000584","name":"mzML format","value":null},',
  '"id_format":{"cv_label":"MS","cv_accession":"MS:1000768","name":"Thermo nativeID format","value":null},',
  '"fragmentation_method":null,',
  '"scan_polarity":[{"id":1,"cv_label":"MS","cv_accession":"MS:1000130","name":"positive scan","value":null}],',
  '"hash":null,"hash_method":null,"parameters":null,"usi_identifier":null}],',
  '"assay":[{"id":1,"name":"Assay 1","custom":null,"external_uri":null,"sample_ref":1,"ms_run_ref":[1],"protocol_refs":null,"parameters":null}],',
  '"study_variable_group":[{"id":1,"name":{"cv_label":"STATO","cv_accession":"STATO:0000087","name":"independent variable","value":null},',
  '"description":"Group","type":{"cv_label":"STATO","cv_accession":"STATO:0000252","name":"categorical variable","value":null},',
  '"datatype":"xsd:string","unit":{"cv_label":"UO","cv_accession":"UO:0000186","name":"dimensionless unit","value":null},"study_variable_refs":[1]}],',
  '"study_variable":[{"id":1,"name":"Group 1","assay_refs":[1],"ms_run_refs":[1],"description":"Group 1",',
  '"average_function":{"cv_label":"MS","cv_accession":"MS:1002883","name":"median","value":null},',
  '"variation_function":{"cv_label":"MS","cv_accession":"MS:1002885","name":"standard error","value":null},"factors":null}],',
  '"custom":null,',
  '"cv":[',
  '{"id":1,"label":"MS","full_name":"PSI-MS","version":"4.1.0","uri":"https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo"},',
  '{"id":2,"label":"MSIO","full_name":"Metabolomics Standards Initiative Ontology","version":"1.0","uri":"https://purl.obolibrary.org/obo/msio.owl"},',
  '{"id":3,"label":"MIRIAM","full_name":"MIRIAM Registry","version":"1.0","uri":"https://identifiers.org/"},',
  '{"id":4,"label":"PRIDE","full_name":"PRIDE CV","version":"1.0","uri":"https://www.ebi.ac.uk/ols/ontologies/pride"},',
  '{"id":5,"label":"STATO","full_name":"STATO","version":"1.4","uri":"http://stato-ontology.org/"},',
  '{"id":6,"label":"UO","full_name":"Units of Measurement Ontology","version":"1.0","uri":"https://purl.obolibrary.org/obo/uo.owl"}],',
  '"small_molecule-quantification_unit":{"cv_label":"PRIDE","cv_accession":"PRIDE:0000393","name":"Relative quantification unit","value":null},',
  '"small_molecule_feature-quantification_unit":null,',
  '"small_molecule-identification_reliability":{"cv_label":"MS","cv_accession":"MS:1002896","name":"compound identification confidence level","value":null},',
  '"database":[{"id":1,"param":{"cv_label":"MIRIAM","cv_accession":"MIR:00100079","name":"HMDB","value":null},"prefix":"HMDB","version":"5.0","uri":"https://hmdb.ca"}],',
  '"id_confidence_measure":null,"colunit-small_molecule":null,"colunit-small_molecule_feature":null,"colunit-small_molecule_evidence":null},',
  '"smallMoleculeSummary":[',
  '{"prefix":"SML","header_prefix":"SMH","sml_id":1,"smf_id_refs":null,"database_identifier":["HMDB:HMDB0000122"],"chemical_formula":["C6H12O6"],',
  '"smiles":null,"inchi":null,"chemical_name":["Glucose"],"uri":null,"theoretical_neutral_mass":[180.0634],"adduct_ions":["[M+H]1+"],',
  '"reliability":"2","best_id_confidence_measure":{"cv_label":"MS","cv_accession":"MS:1002890","name":"fragmentation score","value":null},',
  '"best_id_confidence_value":0.95,"abundance_assay":[100.0],"abundance_study_variable":[100.0],"abundance_variation_study_variable":[0.0]},',
  '{"prefix":"SML","header_prefix":"SMH","sml_id":2,"smf_id_refs":null,"database_identifier":null,"chemical_formula":null,',
  '"smiles":null,"inchi":null,"chemical_name":null,"uri":null,"theoretical_neutral_mass":null,"adduct_ions":null,',
  '"reliability":"4","best_id_confidence_measure":null,"best_id_confidence_value":0.5,',
  '"abundance_assay":[null],"abundance_study_variable":[null],"abundance_variation_study_variable":[null]}],',
  '"smallMoleculeFeature":null,"smallMoleculeEvidence":null,"comment":null}'
)

test_that("the base document is valid", {
  expect_equal(nrow(validate(base)), 0L)
  expect_equal(nrow(validate(charToRaw(base))), 0L)
  expect_equal(nrow(validate(base_json)), 0L)
})

test_that("findings are a data frame", {
  bad <- sub("\t2.1.0-M\n", "\t2.1\n", base, fixed = TRUE)
  findings <- validate(bad)
  expect_equal(findings$code, "MZT-S006")
  expect_equal(findings$severity, "error")
  expect_equal(c(findings$line, findings$column, findings$cell), c(1L, 22L, 3L))
  expect_match(findings$spec, "^https://github.com/HUPO-PSI/mzTab-M/blob/")
  missing <- "MTD\tmzTab-version\t2.1.0-M\n"
  structure <- validate(missing, layer = "structure")
  expect_true("MZT-T009" %in% structure$code)
  expect_true(any(is.na(structure$line)))
  expect_equal(nrow(validate(missing, layer = "syntax")), 0L)
  quiet <- validate(missing, severities = c("MZT-T009" = "off"))
  expect_false("MZT-T009" %in% quiet$code)
  terms <- validate(base, vocabularies = "[Term]\nid: MS:1\nname: x\n")
  expect_true("MZT-X004" %in% terms$code)
  expect_error(validate(base, layer = "all"), "unknown layer")
  expect_error(validate(base, severities = c("MZT-T009" = "loud")), "unknown severity")
  expect_error(validate(c("a", "b")), "one non-NA character string")
})

test_that("documents convert both ways", {
  expect_equal(convert(base, "json"), base_json)
  expect_equal(convert(base_json, "tsv"), base)
  expect_equal(canonical(base), base)
  unknown <- sub("MTD\ttitle\t", "MTD\tnot-a-key\tx\nMTD\ttitle\t", base, fixed = TRUE)
  expect_error(convert(unknown, "json"), "no JSON form")
  expect_equal(convert(unknown, "json", lossy = TRUE), base_json)
  expect_error(convert(base, "xml"), "unknown format")
  expect_equal(canonical(base_json, format = "json"), base_json)
})

test_that("inspect, codes and the spec commit", {
  facts <- inspect(base)
  expect_equal(facts$version, "2.1.0-M")
  expect_equal(facts$sml_rows, 2L)
  expect_true(is.na(inspect("MTD\ttitle\tx\n")$sml_rows))
  table <- codes()
  expect_true("MZT-T009" %in% table$code)
  expect_equal(nchar(spec_commit()), 40L)
  expect_equal(spec_version(), "2.1.0-M")
  expect_equal(vocabulary_size("[Term]\nid: MS:1\n"), 1L)
})

test_that("raw bytes, NAs, and empty format are handled strictly", {
  bad_utf8 <- c(charToRaw("MTD\ttitle\t"), as.raw(c(0xff, 0x0a)))
  utf8_findings <- validate(bad_utf8, layer = "syntax")
  expect_true("MZT-S008" %in% utf8_findings$code)

  with_nul <- c(charToRaw("MTD\ttitle\t"), as.raw(0x00), charToRaw("\n"))
  expect_equal(nrow(validate(with_nul, layer = "syntax")), 0L)

  expect_error(validate(base, format = ""), "unknown format")
  expect_error(validate(base, vocabularies = NA_character_), "NA")
  expect_error(validate(base, severities = c("MZT-T009" = NA_character_)), "NA")
  expect_error(validate(base, severities = c("error")), "named")
  expect_error(validate(base, severities = c("MZT-T009=off" = "off")), "unknown code")
  expect_error(validate(NA_character_), "one non-NA character string")
  expect_error(inspect(base, format = NA_character_), "non-NA")
})
