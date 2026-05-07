use anyhow::Result;
use std::path::Path;
use tantivy::{
    collector::TopDocs,
    query::QueryParser,
    schema::{Field, OwnedValue, Schema, STORED, STRING, TEXT},
    Index, IndexReader, IndexWriter, ReloadPolicy,
};

use crate::notes::SearchResult;

fn owned_str(v: &OwnedValue) -> Option<&str> {
    if let OwnedValue::Str(s) = v { Some(s) } else { None }
}

pub struct FtsIndex {
    index: Index,
    reader: IndexReader,
    f_path: Field,
    f_title: Field,
    f_body: Field,
    f_tags: Field,
}

impl FtsIndex {
    pub fn open_or_create(index_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(index_dir)?;

        let mut schema_builder = Schema::builder();
        let f_path = schema_builder.add_text_field("path", STRING | STORED);
        let f_title = schema_builder.add_text_field("title", TEXT | STORED);
        let f_body = schema_builder.add_text_field("body", TEXT);
        let f_tags = schema_builder.add_text_field("tags", TEXT | STORED);
        let schema = schema_builder.build();

        let index = if index_dir.join("meta.json").exists() {
            Index::open_in_dir(index_dir)?
        } else {
            Index::create_in_dir(index_dir, schema)?
        };

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(FtsIndex {
            index,
            reader,
            f_path,
            f_title,
            f_body,
            f_tags,
        })
    }

    pub fn index_note(
        &self,
        path: &str,
        title: &str,
        body: &str,
        tags: &[String],
    ) -> Result<()> {
        let mut writer: IndexWriter = self.index.writer(50_000_000)?;

        // Delete any existing doc for this path
        let path_term = tantivy::Term::from_field_text(self.f_path, path);
        writer.delete_term(path_term);

        let mut doc = tantivy::TantivyDocument::default();
        doc.add_text(self.f_path, path);
        doc.add_text(self.f_title, title);
        doc.add_text(self.f_body, body);
        doc.add_text(self.f_tags, &tags.join(" "));

        writer.add_document(doc)?;
        writer.commit()?;
        Ok(())
    }

    pub fn delete_note(&self, path: &str) -> Result<()> {
        let mut writer: IndexWriter = self.index.writer(50_000_000)?;
        let path_term = tantivy::Term::from_field_text(self.f_path, path);
        writer.delete_term(path_term);
        writer.commit()?;
        Ok(())
    }

    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.f_title, self.f_body, self.f_tags],
        );

        let query = query_parser
            .parse_query(query_str)
            .unwrap_or_else(|_| {
                // Fallback: treat as literal phrase
                query_parser
                    .parse_query(&format!("\"{}\"", query_str.replace('"', "")))
                    .unwrap_or_else(|_| Box::new(tantivy::query::AllQuery))
            });

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            let path = doc
                .get_first(self.f_path)
                .and_then(owned_str)
                .unwrap_or("")
                .to_string();
            let title = doc
                .get_first(self.f_title)
                .and_then(owned_str)
                .unwrap_or(&path)
                .to_string();

            results.push(SearchResult {
                relative_path: path.clone(),
                title,
                snippet: String::new(),
                score,
            });
        }

        Ok(results)
    }
}
