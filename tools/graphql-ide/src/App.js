import React, { useState, useEffect } from 'react';
import { GraphQLEditor } from 'graphql-editor';
import './App.css';

// Import default schema as a string
import defaultSchemaText from './schema/pcf-schema';

function App() {
  const [mySchema, setMySchema] = useState({
    code: defaultSchemaText,
    libraries: ''
  });
  const [schemaName, setSchemaName] = useState('PCF Schema');

  // Handle file upload
  const handleFileUpload = (event) => {
    const file = event.target.files[0];
    if (file) {
      setSchemaName(file.name.replace('.graphql', '').replace('.gql', ''));
      const reader = new FileReader();
      reader.onload = (e) => {
        setMySchema({
          code: e.target.result,
          libraries: ''
        });
      };
      reader.readAsText(file);
    }
  };

  // Handle download
  const handleDownload = () => {
    const blob = new Blob([mySchema.code], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${schemaName}.graphql`;
    a.click();
    URL.revokeObjectURL(url);
  };

  // Reset to default schema
  const handleReset = () => {
    setMySchema({
      code: defaultSchemaText,
      libraries: ''
    });
    setSchemaName('PCF Schema');
  };

  return (
    <div className="App">
      <header className="App-header">
        <h3>{schemaName}</h3>
        <div className="actions">
          <label className="button button-primary">
            <input
              type="file"
              accept=".graphql,.gql"
              onChange={handleFileUpload}
              style={{ display: 'none' }}
            />
            Load Schema
          </label>
          <button 
            className="button button-success" 
            onClick={handleDownload}
          >
            Download
          </button>
          <button 
            className="button button-secondary" 
            onClick={handleReset}
          >
            Reset to Default
          </button>
        </div>
      </header>
      <div className="editor-container">
        <GraphQLEditor
          setSchema={(props) => setMySchema(props)}
          schema={mySchema}
        />
      </div>
    </div>
  );
}

export default App;
