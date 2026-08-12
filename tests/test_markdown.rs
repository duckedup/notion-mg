use notion_mg::markdown::{MAX_BLOCKS_PER_REQUEST, chunk_blocks, markdown_to_blocks, parse_inline};
use serde_json::{Value, json};

fn types(blocks: &[Value]) -> Vec<&str> {
    blocks.iter().map(|b| b["type"].as_str().unwrap()).collect()
}

fn body(block: &Value) -> &Value {
    &block[block["type"].as_str().unwrap()]
}

fn text_of(block: &Value) -> String {
    body(block)["rich_text"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rt| rt["text"]["content"].as_str().unwrap())
        .collect()
}

#[test]
fn headings_map_to_notion_levels() {
    let blocks = markdown_to_blocks("# One\n\n## Two\n\n### Three\n\n#### Four\n\n###### Six");
    assert_eq!(
        types(&blocks),
        [
            "heading_1",
            "heading_2",
            "heading_3",
            "heading_3",
            "heading_3"
        ]
    );
    assert_eq!(text_of(&blocks[3]), "Four");
}

#[test]
fn hash_without_space_is_not_a_heading() {
    let blocks = markdown_to_blocks("#hashtag not a heading");
    assert_eq!(types(&blocks), ["paragraph"]);
}

#[test]
fn paragraph_lines_join_until_a_blank_line() {
    let blocks = markdown_to_blocks("one\ntwo\n\nthree");
    assert_eq!(types(&blocks), ["paragraph", "paragraph"]);
    assert_eq!(text_of(&blocks[0]), "one\ntwo");
    assert_eq!(text_of(&blocks[1]), "three");
}

#[test]
fn paragraph_stops_at_the_next_block() {
    let blocks = markdown_to_blocks("intro text\n# heading");
    assert_eq!(types(&blocks), ["paragraph", "heading_1"]);
}

#[test]
fn inline_annotations_are_applied() {
    let rich = parse_inline("plain **bold** *italic* `code` ~~struck~~");

    let annotated: Vec<(&str, bool, bool, bool, bool)> = rich
        .iter()
        .map(|rt| {
            let a = &rt["annotations"];
            (
                rt["text"]["content"].as_str().unwrap(),
                a["bold"].as_bool().unwrap_or(false),
                a["italic"].as_bool().unwrap_or(false),
                a["code"].as_bool().unwrap_or(false),
                a["strikethrough"].as_bool().unwrap_or(false),
            )
        })
        .collect();

    assert!(annotated.contains(&("bold", true, false, false, false)));
    assert!(annotated.contains(&("italic", false, true, false, false)));
    assert!(annotated.contains(&("code", false, false, true, false)));
    assert!(annotated.contains(&("struck", false, false, false, true)));
    assert_eq!(annotated[0].0, "plain ");
    assert!(rich[0].get("annotations").is_none());
}

#[test]
fn nested_emphasis_combines_annotations() {
    let rich = parse_inline("**bold and *also italic***");
    let inner = rich
        .iter()
        .find(|rt| rt["text"]["content"] == "also italic")
        .expect("nested span");
    assert_eq!(inner["annotations"]["bold"], json!(true));
    assert_eq!(inner["annotations"]["italic"], json!(true));
}

#[test]
fn links_become_rich_text_links() {
    let rich = parse_inline("see [the docs](https://example.com/a_b) now");
    let link = rich
        .iter()
        .find(|rt| rt["text"]["content"] == "the docs")
        .expect("link span");
    assert_eq!(
        link["text"]["link"]["url"],
        json!("https://example.com/a_b")
    );
}

#[test]
fn link_titles_are_dropped_from_the_url() {
    let rich = parse_inline("[x](https://example.com \"Title\")");
    assert_eq!(rich[0]["text"]["link"]["url"], json!("https://example.com"));
}

#[test]
fn underscores_inside_words_are_literal() {
    let rich = parse_inline("call snake_case_name here");
    assert_eq!(rich.len(), 1);
    assert_eq!(
        rich[0]["text"]["content"],
        json!("call snake_case_name here")
    );
    assert!(rich[0].get("annotations").is_none());
}

#[test]
fn underscores_at_word_boundaries_still_emphasize() {
    let rich = parse_inline("_italic_ text");
    assert_eq!(rich[0]["text"]["content"], json!("italic"));
    assert_eq!(rich[0]["annotations"]["italic"], json!(true));
}

#[test]
fn backslash_escapes_delimiters() {
    let rich = parse_inline(r"literal \*not italic\* here");
    assert_eq!(rich.len(), 1);
    assert_eq!(
        rich[0]["text"]["content"],
        json!("literal *not italic* here")
    );
}

#[test]
fn code_spans_are_not_parsed_for_emphasis() {
    let rich = parse_inline("`a * b * c`");
    assert_eq!(rich.len(), 1);
    assert_eq!(rich[0]["text"]["content"], json!("a * b * c"));
    assert_eq!(rich[0]["annotations"]["code"], json!(true));
}

#[test]
fn unmatched_delimiter_stays_literal() {
    let rich = parse_inline("2 * 3 = 6");
    assert_eq!(rich.len(), 1);
    assert_eq!(rich[0]["text"]["content"], json!("2 * 3 = 6"));
}

#[test]
fn bulleted_and_numbered_lists() {
    let blocks = markdown_to_blocks("- one\n- two\n\n1. first\n2. second");
    assert_eq!(
        types(&blocks),
        [
            "bulleted_list_item",
            "bulleted_list_item",
            "numbered_list_item",
            "numbered_list_item"
        ]
    );
    assert_eq!(text_of(&blocks[3]), "second");
}

#[test]
fn task_list_items_carry_checked_state() {
    let blocks = markdown_to_blocks("- [x] done\n- [ ] pending");
    assert_eq!(types(&blocks), ["to_do", "to_do"]);
    assert_eq!(body(&blocks[0])["checked"], json!(true));
    assert_eq!(body(&blocks[1])["checked"], json!(false));
}

#[test]
fn indentation_becomes_nesting() {
    let blocks = markdown_to_blocks("- top\n  - child\n    - grandchild\n- sibling");
    assert_eq!(blocks.len(), 2);

    let child = &body(&blocks[0])["children"][0];
    assert_eq!(text_of(child), "child");

    let grandchild = &body(child)["children"][0];
    assert_eq!(text_of(grandchild), "grandchild");
    assert_eq!(text_of(&blocks[1]), "sibling");
}

#[test]
fn nested_children_may_change_list_type() {
    let blocks = markdown_to_blocks("- bullet\n  1. numbered child");
    let child = &body(&blocks[0])["children"][0];
    assert_eq!(child["type"], json!("numbered_list_item"));
}

#[test]
fn a_blank_line_does_not_split_a_list() {
    let blocks = markdown_to_blocks("- one\n\n- two");
    assert_eq!(types(&blocks), ["bulleted_list_item", "bulleted_list_item"]);
}

#[test]
fn fenced_code_keeps_content_verbatim() {
    let blocks = markdown_to_blocks("```python\nif x:\n    y = '**not bold**'\n```");
    assert_eq!(types(&blocks), ["code"]);
    assert_eq!(body(&blocks[0])["language"], json!("python"));
    assert_eq!(text_of(&blocks[0]), "if x:\n    y = '**not bold**'");
}

#[test]
fn code_language_aliases_resolve() {
    for (tag, expected) in [
        ("rs", "rust"),
        ("js", "javascript"),
        ("ts", "typescript"),
        ("yml", "yaml"),
        ("sh", "shell"),
        ("", "plain text"),
        ("brainfuck", "plain text"),
    ] {
        let blocks = markdown_to_blocks(&format!("```{tag}\nx\n```"));
        assert_eq!(
            body(&blocks[0])["language"],
            json!(expected),
            "language tag {tag:?}"
        );
    }
}

#[test]
fn unterminated_code_fence_still_produces_a_block() {
    let blocks = markdown_to_blocks("```\nno closing fence");
    assert_eq!(types(&blocks), ["code"]);
    assert_eq!(text_of(&blocks[0]), "no closing fence");
}

#[test]
fn quotes_merge_consecutive_lines() {
    let blocks = markdown_to_blocks("> first\n> second\n\nafter");
    assert_eq!(types(&blocks), ["quote", "paragraph"]);
    assert_eq!(text_of(&blocks[0]), "first\nsecond");
}

#[test]
fn dividers_are_recognized() {
    let blocks = markdown_to_blocks("a\n\n---\n\nb\n\n***\n\nc");
    assert_eq!(
        types(&blocks),
        ["paragraph", "divider", "paragraph", "divider", "paragraph"]
    );
}

#[test]
fn standalone_image_becomes_an_image_block() {
    let blocks = markdown_to_blocks("![A cat](https://example.com/cat.png)");
    assert_eq!(types(&blocks), ["image"]);
    let image = body(&blocks[0]);
    assert_eq!(
        image["external"]["url"],
        json!("https://example.com/cat.png")
    );
    assert_eq!(image["caption"][0]["text"]["content"], json!("A cat"));
}

#[test]
fn image_with_surrounding_text_stays_a_paragraph() {
    let blocks = markdown_to_blocks("see ![A cat](https://example.com/cat.png) there");
    assert_eq!(types(&blocks), ["paragraph"]);
}

#[test]
fn gfm_tables_become_table_blocks() {
    let blocks = markdown_to_blocks("| A | B |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |");
    assert_eq!(types(&blocks), ["table"]);

    let table = body(&blocks[0]);
    assert_eq!(table["table_width"], json!(2));
    assert_eq!(table["has_column_header"], json!(true));

    let rows = table["children"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows[0]["table_row"]["cells"][0][0]["text"]["content"],
        json!("A")
    );
    assert_eq!(
        rows[2]["table_row"]["cells"][1][0]["text"]["content"],
        json!("4")
    );
}

#[test]
fn short_table_rows_are_padded_to_the_header_width() {
    let blocks = markdown_to_blocks("| A | B |\n| --- | --- |\n| 1 |");
    let rows = body(&blocks[0])["children"].as_array().unwrap();
    assert_eq!(rows[1]["table_row"]["cells"].as_array().unwrap().len(), 2);
}

#[test]
fn a_pipe_line_without_a_delimiter_row_is_a_paragraph() {
    let blocks = markdown_to_blocks("| not | a table |");
    assert_eq!(types(&blocks), ["paragraph"]);
}

#[test]
fn long_text_splits_across_rich_text_objects() {
    let long = "x".repeat(4500);
    let rich = parse_inline(&long);
    assert_eq!(rich.len(), 3);
    assert_eq!(rich[0]["text"]["content"].as_str().unwrap().len(), 2000);
    assert_eq!(rich[2]["text"]["content"].as_str().unwrap().len(), 500);
}

#[test]
fn multibyte_text_splits_on_character_boundaries() {
    let long = "é".repeat(2500);
    let rich = parse_inline(&long);
    assert_eq!(rich.len(), 2);
    assert_eq!(
        rich[0]["text"]["content"].as_str().unwrap().chars().count(),
        2000
    );
}

#[test]
fn blocks_chunk_to_the_request_limit() {
    let blocks = markdown_to_blocks(&"- item\n".repeat(250));
    assert_eq!(blocks.len(), 250);

    let chunks = chunk_blocks(blocks);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].len(), MAX_BLOCKS_PER_REQUEST);
    assert_eq!(chunks[2].len(), 50);
}

#[test]
fn empty_input_produces_no_blocks() {
    assert!(markdown_to_blocks("").is_empty());
    assert!(markdown_to_blocks("\n\n   \n").is_empty());
}

#[test]
fn every_block_is_tagged_for_the_notion_api() {
    let blocks = markdown_to_blocks("# h\n\ntext\n\n- item\n\n> quote\n\n```\ncode\n```\n\n---");
    for block in &blocks {
        assert_eq!(block["object"], json!("block"));
        let block_type = block["type"].as_str().unwrap();
        assert!(block.get(block_type).is_some(), "missing {block_type} body");
    }
}
