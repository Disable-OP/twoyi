const ZAI = require('z-ai-web-dev-sdk').default;
const fs = require('fs');

async function main() {
    const imagePath = process.argv[2];
    const prompt = process.argv[3] || "Describe what you see in this Android screenshot.";
    const model = process.argv[4] || "glm-4.6v";

    const imgBuf = fs.readFileSync(imagePath);
    const imgB64 = imgBuf.toString('base64');

    const zai = await ZAI.create();
    const result = await zai.chat.completions.createVision({
        model: model,
        messages: [
            {
                role: "user",
                content: [
                    { type: "image_url", image_url: { url: `data:image/png;base64,${imgB64}` } },
                    { type: "text", text: prompt }
                ]
            }
        ],
        thinking: { type: "disabled" }
    });

    console.log(result.choices[0].message.content);
}

main().catch(e => { console.error(e); process.exit(1); });
