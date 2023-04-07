import React from 'react';
import './App.css';

interface RpcRequest {
  type: string;
  function: string;
  arguments: string[];
  type_arguments: string[];
}

interface Address {
  hex: string
}

function App() {
  let eventSource: EventSource | null = null;
  const [address, setAddress] = React.useState<string | null>(null);
  const params = new URLSearchParams(window.location.search);

  const init = async () => {
    const {address} = await window.aptos.connect();
    setAddress(address);
    
    if (params.has('payload')) {
      eventSource = new EventSource('sse');
      if (eventSource !== null) {
        eventSource.addEventListener('payload', async (event) => {  
          if (eventSource !== null) {
            eventSource.close();
          }
          try {
            const payload: RpcRequest = JSON.parse(event.data);
            await window.aptos.signAndSubmitTransaction(payload);
            window.close();
          } catch (e) {
            console.log(e);
          }
        });
      }
    }

    if (params.has('address') && address !== null) {
      try {
        await fetch('http://127.0.0.1:8080/address', {
          method: 'POST',
          mode: 'same-origin',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({hex: address}) 
        });
        window.close();
      } catch (e) {
        console.log(e);
      }
    }
  }

  React.useEffect(() => {
    init();
  }, []);

  return (
    <div className="App">
      <header className="App-header">
        <p>
          This window will be closed automatically.<br/>
          If not, please close manually.
        </p>
      </header>
    </div>
  );
}

export default App;
